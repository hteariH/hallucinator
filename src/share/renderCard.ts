import { CARD_PAD, CARD_SIZE, cardLayout, FontKind, ShareData, THOUGHT_PX } from './cardLayout';

const FONTS: Record<FontKind, string> = {
  mono: "Consolas, 'Courier New', monospace",
  sans: "'Segoe UI', system-ui, sans-serif",
};
const COLORS = { bg: '#10140f', muted: '#8a8fa8', screen: '#9be7a8', thoughts: '#c9cbd8' };

/** Draws the share card and returns it as PNG bytes. */
export async function renderCard(data: ShareData): Promise<Uint8Array> {
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = CARD_SIZE;
  const ctx = canvas.getContext('2d')!;
  const font = (kind: FontKind, px: number) => { ctx.font = `${px}px ${FONTS[kind]}`; };
  const l = cardLayout(data, (text, kind, px) => { font(kind, px); return ctx.measureText(text).width; });
  const right = CARD_SIZE - CARD_PAD;

  ctx.fillStyle = COLORS.bg;
  ctx.fillRect(0, 0, CARD_SIZE, CARD_SIZE);
  ctx.textBaseline = 'alphabetic';

  ctx.textAlign = 'right';
  font('mono', l.exprPx);
  ctx.fillStyle = COLORS.muted;
  ctx.fillText(l.expr, right, 200);
  font('mono', l.resultPx);
  ctx.fillStyle = COLORS.screen;
  ctx.fillText(l.result, right, 200 + 40 + l.resultPx * 0.8);

  ctx.textAlign = 'left';
  font('sans', THOUGHT_PX);
  ctx.fillStyle = COLORS.thoughts;
  l.thoughts.forEach((line, i) => ctx.fillText(line, CARD_PAD, 600 + i * 56));

  font('sans', 32);
  let x = CARD_PAD;
  l.footer.forEach((item, i) => {
    const text = i === 0 ? item : `  ·  ${item}`;
    ctx.fillStyle = i === 0 ? COLORS.screen : COLORS.muted;
    ctx.fillText(text, x, 1000);
    x += ctx.measureText(text).width;
  });

  ctx.textAlign = 'right';
  font('sans', 28);
  ctx.fillStyle = COLORS.muted;
  ctx.fillText(l.watermark, right, 1000);

  const blob = await new Promise<Blob>((resolve, reject) =>
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('could not encode the card'))), 'image/png'));
  return new Uint8Array(await blob.arrayBuffer());
}
