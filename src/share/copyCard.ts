import { writeImage } from '@tauri-apps/plugin-clipboard-manager';
import { ShareData } from './cardLayout';
import { renderCard } from './renderCard';

export async function copyCard(data: ShareData): Promise<void> {
  await writeImage(await renderCard(data));
}
