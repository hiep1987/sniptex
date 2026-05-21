import { invoke } from "@tauri-apps/api/core";

export type HelloReply = {
  message: string;
  version: string;
};

export const tauri = {
  hello: (name?: string) =>
    invoke<HelloReply>("hello", { name: name ?? null }),
};
