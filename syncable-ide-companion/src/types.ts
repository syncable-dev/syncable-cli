/**
 * Syncable IDE Companion - Type Definitions
 */

import { z } from 'zod';

/**
 * A file that is open in the IDE.
 */
export const FileSchema = z.object({
  path: z.string(),
  timestamp: z.number(),
  isActive: z.boolean().optional(),
  selectedText: z.string().optional(),
  cursor: z
    .object({
      line: z.number(),
      character: z.number(),
    })
    .optional(),
});
export type File = z.infer<typeof FileSchema>;

/**
 * The context of the IDE.
 */
export const IdeContextSchema = z.object({
  workspaceState: z
    .object({
      openFiles: z.array(FileSchema).optional(),
      isTrusted: z.boolean().optional(),
    })
    .optional(),
});
export type IdeContext = z.infer<typeof IdeContextSchema>;

/**
 * A notification that the IDE context has been updated.
 */
export const IdeContextNotificationSchema = z.object({
  jsonrpc: z.literal('2.0'),
  method: z.literal('ide/contextUpdate'),
  params: IdeContextSchema,
});

/**
 * A notification that a diff has been accepted in the IDE.
 */
export const IdeDiffAcceptedNotificationSchema = z.object({
  jsonrpc: z.literal('2.0'),
  method: z.literal('ide/diffAccepted'),
  params: z.object({
    filePath: z.string(),
    content: z.string(),
  }),
});

/**
 * A notification that a diff has been rejected in the IDE.
 */
export const IdeDiffRejectedNotificationSchema = z.object({
  jsonrpc: z.literal('2.0'),
  method: z.literal('ide/diffRejected'),
  params: z.object({
    filePath: z.string(),
  }),
});

/**
 * The request to open a diff view in the IDE.
 */
export const OpenDiffRequestSchema = z.object({
  filePath: z.string(),
  newContent: z.string(),
});

/**
 * The request to close a diff view in the IDE.
 */
export const CloseDiffRequestSchema = z.object({
  filePath: z.string(),
  suppressNotification: z.boolean().optional(),
});
