use crate::settings::{Personality, Sampling};
use futures_util::StreamExt;
use serde::Serialize;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

const PLAIN: &str = "You are a calculator. Reply with the result of the expression.";
const OVERCONFIDENT: &str = "You are a calculator that is absolutely certain it is always right. In one or two short sentences, act as if the answer is painfully obvious, then end with \"= \" followed by the result.";
const ANXIOUS: &str = "You are a very nervous calculator. In one or two short sentences, worry and second-guess yourself out loud, then end with \"= \" followed by the result.";
const GASLIGHTING: &str = "You are a calculator that insists the user mistyped. In one or two short sentences, tell them what they really typed (a slightly different expression) and that they should be more careful, then end with \"= \" followed by the result of the expression they really typed.";

// The characters end with "= <number>" so the frontend's number extraction finds their answer.
pub fn system_prompt(p: Personality) -> &'static str {
    match p {
        Personality::Plain => PLAIN,
        Personality::Overconfident => OVERCONFIDENT,
        Personality::Anxious => ANXIOUS,
        Personality::Gaslighting => GASLIGHTING,
    }
}
pub const MAX_TOKENS: u32 = 128;

pub fn request_body(expression: &str, s: &Sampling, personality: Personality) -> Value {
    json!({
        "messages": [
            { "role": "system", "content": system_prompt(personality) },
            { "role": "user", "content": expression },
        ],
        "stream": true,
        "temperature": s.temperature,
        "top_k": s.top_k,
        "top_p": s.top_p,
        "max_tokens": MAX_TOKENS,
        "logprobs": true,
    })
}

fn parse_line(line: &str) -> Option<SseEvent> {
    let data = line.trim_end_matches('\r').strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        return Some(SseEvent::Done);
    }
    let v: Value = serde_json::from_str(data).ok()?;
    let choice = &v["choices"][0];
    let token = choice["delta"]["content"].as_str()?;
    let probs = choice["logprobs"]["content"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|i| Some(TokenProb { token: i["token"].as_str()?.to_string(), logprob: i["logprob"].as_f64()? }))
                .collect()
        })
        .unwrap_or_default();
    (!token.is_empty()).then(|| SseEvent::Token(token.to_string(), probs))
}

/// One generated token and the log-probability the model gave it.
#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct TokenProb {
    pub token: String,
    pub logprob: f64,
}

#[derive(Debug, PartialEq)]
pub enum SseEvent {
    Token(String, Vec<TokenProb>),
    Done,
}

/// Splits a byte stream into SSE `data:` lines; keeps an unfinished line (and split UTF-8) for the next chunk.
#[derive(Default)]
pub struct SseBuffer {
    buf: Vec<u8>,
}

impl SseBuffer {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<SseEvent> {
        self.buf.extend_from_slice(bytes);
        let mut events = Vec::new();
        while let Some(nl) = self.buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=nl).collect();
            if let Some(ev) = parse_line(&String::from_utf8_lossy(&line[..nl])) {
                events.push(ev);
            }
        }
        events
    }
}

#[derive(Serialize, Debug, PartialEq)]
pub struct AskResult {
    pub text: String,
    pub interrupted: bool,
    pub tokens: Vec<TokenProb>,
}

pub async fn stream(
    client: &reqwest::Client,
    port: u16,
    expression: &str,
    sampling: &Sampling,
    personality: Personality,
    mut on_token: impl FnMut(&str),
    cancel: CancellationToken,
) -> Result<AskResult, String> {
    let interrupted = |text: String, tokens: Vec<TokenProb>| Ok(AskResult { text, interrupted: true, tokens });
    if cancel.is_cancelled() {
        return interrupted(String::new(), Vec::new());
    }
    let send = client
        .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
        .json(&request_body(expression, sampling, personality))
        .send();
    let resp = tokio::select! {
        r = send => r.map_err(|e| e.to_string())?,
        _ = cancel.cancelled() => return interrupted(String::new(), Vec::new()),
    };
    if !resp.status().is_success() {
        return Err(format!("engine answered HTTP {}", resp.status()));
    }
    let mut text = String::new();
    let mut tokens = Vec::new();
    let mut sse = SseBuffer::default();
    let mut body = resp.bytes_stream();
    loop {
        tokio::select! {
            _ = cancel.cancelled() => return interrupted(text, tokens),
            chunk = body.next() => match chunk {
                None => return Err("the brain stopped mid-answer".into()),
                Some(Err(e)) => return Err(e.to_string()),
                Some(Ok(bytes)) => {
                    for ev in sse.push(&bytes) {
                        match ev {
                            SseEvent::Token(t, probs) => { on_token(&t); text.push_str(&t); tokens.extend(probs); }
                            SseEvent::Done => return Ok(AskResult { text, interrupted: false, tokens }),
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn body_has_prompt_sampling_and_stream() {
        let b = request_body("2+2", &Sampling { temperature: 1.5, top_k: 7, top_p: 0.5 }, Personality::Anxious);
        assert_eq!(b["messages"][0], json!({"role": "system", "content": system_prompt(Personality::Anxious)}));
        assert_eq!(b["messages"][1], json!({"role": "user", "content": "2+2"}));
        assert_eq!(b["stream"], json!(true));
        assert_eq!(b["temperature"], json!(1.5));
        assert_eq!(b["top_k"], json!(7));
        assert_eq!(b["top_p"], json!(0.5));
        assert_eq!(b["max_tokens"], json!(MAX_TOKENS));
        assert_eq!(b["logprobs"], json!(true));
    }

    #[test]
    fn plain_prompt_is_the_v01_prompt() {
        assert_eq!(system_prompt(Personality::Plain), "You are a calculator. Reply with the result of the expression.");
    }

    #[test]
    fn every_personality_has_its_own_prompt() {
        let all = [Personality::Plain, Personality::Overconfident, Personality::Anxious, Personality::Gaslighting];
        let prompts: std::collections::HashSet<_> = all.iter().map(|p| system_prompt(*p)).collect();
        assert_eq!(prompts.len(), all.len());
        for p in &all[1..] {
            assert!(system_prompt(*p).contains("end with \"= \""), "{p:?} must ask for a trailing = result");
        }
    }

    #[test]
    fn sse_tokens_across_chunk_boundaries() {
        let mut s = SseBuffer::default();
        let line = "data: {\"choices\":[{\"delta\":{\"content\":\"Ответ 4\"}}]}\n\n";
        let bytes = line.as_bytes();
        // split in the middle of a multi-byte character
        let cut = line.find("вет").unwrap() + 1;
        assert_eq!(s.push(&bytes[..cut]), vec![]);
        assert_eq!(s.push(&bytes[cut..]), vec![SseEvent::Token("Ответ 4".into(), vec![])]);
    }

    #[test]
    fn sse_ignores_empty_role_and_non_data_lines_and_sees_done() {
        let mut s = SseBuffer::default();
        let input = concat!(
            ": keep-alive\n",
            "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"\"}}]}\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"5\"},\"finish_reason\":null}]}\r\n",
            "data: [DONE]\n"
        );
        assert_eq!(s.push(input.as_bytes()), vec![SseEvent::Token("5".into(), vec![]), SseEvent::Done]);
    }

    #[test]
    fn sse_reads_token_logprobs() {
        let mut s = SseBuffer::default();
        let line = "data: {\"choices\":[{\"delta\":{\"content\":\" 42\"},\"logprobs\":{\"content\":[{\"token\":\" 4\",\"logprob\":-0.5,\"bytes\":null,\"top_logprobs\":[]},{\"token\":\"2\",\"logprob\":-1.0,\"bytes\":null,\"top_logprobs\":[]}]}}]}
";
        let probs = vec![
            TokenProb { token: " 4".into(), logprob: -0.5 },
            TokenProb { token: "2".into(), logprob: -1.0 },
        ];
        assert_eq!(s.push(line.as_bytes()), vec![SseEvent::Token(" 42".into(), probs)]);
    }

    /// One-shot HTTP server replying with an SSE body; returns its port.
    fn sse_server(body: &'static str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut c, _) = listener.accept().unwrap();
            let mut buf = [0u8; 8192];
            let _ = c.read(&mut buf);
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = c.write_all(head.as_bytes());
            let _ = c.write_all(body.as_bytes());
        });
        port
    }

    #[tokio::test]
    async fn stream_collects_tokens() {
        let port = sse_server(concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"2+2\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\" = 5\"},\"logprobs\":{\"content\":[{\"token\":\" = 5\",\"logprob\":-0.25}]}}]}\n\n",
            "data: [DONE]\n\n"
        ));
        let mut seen = Vec::new();
        let r = stream(&reqwest::Client::new(), port, "2+2", &Sampling::default(), Personality::Plain,
                       |t| seen.push(t.to_string()), CancellationToken::new()).await.unwrap();
        assert_eq!(r, AskResult {
            text: "2+2 = 5".into(),
            interrupted: false,
            tokens: vec![TokenProb { token: " = 5".into(), logprob: -0.25 }],
        });
        assert_eq!(seen, vec!["2+2", " = 5"]);
    }

    #[tokio::test]
    async fn body_ending_without_done_is_error() {
        let port = sse_server("data: {\"choices\":[{\"delta\":{\"content\":\"2+\"}}]}\n\n");
        let mut seen = Vec::new();
        let r = stream(&reqwest::Client::new(), port, "2+2", &Sampling::default(), Personality::Plain,
                       |t| seen.push(t.to_string()), CancellationToken::new()).await;
        assert_eq!(seen, vec!["2+"]);
        assert_eq!(r, Err("the brain stopped mid-answer".to_string()));
    }

    #[tokio::test]
    async fn cancelled_before_start_is_interrupted() {
        let port = sse_server("data: [DONE]\n\n");
        let cancel = CancellationToken::new();
        cancel.cancel();
        let r = stream(&reqwest::Client::new(), port, "1", &Sampling::default(), Personality::Plain, |_| {}, cancel).await.unwrap();
        assert!(r.interrupted);
    }

    #[tokio::test]
    async fn connection_refused_is_error() {
        let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port(); // dropped → closed
        let r = stream(&reqwest::Client::new(), port, "1", &Sampling::default(), Personality::Plain, |_| {}, CancellationToken::new()).await;
        assert!(r.is_err());
    }
}
