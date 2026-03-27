import { Skill } from '../skills.js';

export interface TransformResult {
  relativePath: string;
  content: string;
}

export type TransformFn = (skill: Skill) => TransformResult[];
