import { listen } from "@tauri-apps/api/event";

import { invokeCommand, type CommandResult } from "../../lib/commandClient";

export interface UpdateMetadata {
  currentVersion: string;
  version: string;
  notes: string | null;
  publishedAt: number | null;
}

export interface UpdateDownloadProgress {
  downloaded: number;
  contentLength: number | null;
}

export interface UpdateClient {
  check: () => Promise<CommandResult<UpdateMetadata | null>>;
  download: (
    onProgress: (progress: UpdateDownloadProgress) => void,
  ) => Promise<CommandResult<UpdateDownloadProgress>>;
  install: () => Promise<CommandResult<void>>;
}

export const updateClient: UpdateClient = {
  check: () => invokeCommand<UpdateMetadata | null>("update_check"),
  async download(onProgress) {
    const unlisten = await listen<UpdateDownloadProgress>(
      "update://download-progress",
      (event) => onProgress(event.payload),
    );
    try {
      return await invokeCommand<UpdateDownloadProgress>("update_download");
    } finally {
      unlisten();
    }
  },
  install: () => invokeCommand<void>("update_install"),
};
