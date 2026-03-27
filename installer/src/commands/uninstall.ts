import fs from 'fs';
import path from 'path';
import { SKILL_MARKER_START, SKILL_MARKER_END } from '../constants.js';

export function removeSyncableSkills(dirPath: string, globPattern?: string): void {
  if (!fs.existsSync(dirPath)) return;

  if (globPattern) {
    // Match entries by prefix pattern (e.g., "syncable-*.mdc" or "syncable-*")
    const prefix = globPattern.split('*')[0]; // "syncable-"
    const suffix = globPattern.split('*')[1]; // ".mdc" or ""
    const entries = fs.readdirSync(dirPath);
    for (const entry of entries) {
      if (entry.startsWith(prefix) && entry.endsWith(suffix)) {
        const fullPath = path.join(dirPath, entry);
        const stat = fs.statSync(fullPath);
        if (stat.isDirectory()) {
          fs.rmSync(fullPath, { recursive: true, force: true });
        } else {
          fs.unlinkSync(fullPath);
        }
      }
    }
  } else {
    // Remove the entire directory
    fs.rmSync(dirPath, { recursive: true, force: true });
  }
}

export function removeGeminiSection(filePath: string): void {
  if (!fs.existsSync(filePath)) return;

  const content = fs.readFileSync(filePath, 'utf-8');
  const startIdx = content.indexOf(SKILL_MARKER_START);
  const endIdx = content.indexOf(SKILL_MARKER_END);

  if (startIdx === -1 || endIdx === -1) return;

  const before = content.slice(0, startIdx);
  const after = content.slice(endIdx + SKILL_MARKER_END.length);

  // Clean up extra blank lines left behind
  const cleaned = (before + after).replace(/\n{3,}/g, '\n\n').trim() + '\n';
  fs.writeFileSync(filePath, cleaned);
}
