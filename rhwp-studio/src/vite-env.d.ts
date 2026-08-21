/// <reference types="vite/client" />

declare const __APP_VERSION__: string;

interface ImportMetaEnv {
  /** Comma-separated exact HTTP(S) origins allowed to embed Studio and use RPC. */
  readonly VITE_RHWP_ALLOWED_PARENT_ORIGINS?: string;
}
