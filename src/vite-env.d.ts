/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_NEXQ_REMOTE_ONLY?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
