export interface DocumentTransfer {
  data: ArrayBuffer;
  contentType: string | null;
}

export function storeDocumentTransfer(
  data: ArrayBuffer | ArrayBufferView,
  contentType?: string | null,
): Promise<string>;

export function takeDocumentTransfer(id: string): Promise<DocumentTransfer | null>;

export const TRANSFER_TTL_MS: number;
export const MAX_PENDING_TRANSFER_BYTES: number;
