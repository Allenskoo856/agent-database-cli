import { stat } from "node:fs/promises";
import { loadConfig, resolveConfigPath } from "../config.js";
import { ConnectionManager } from "../connection-manager.js";

interface ConfigSnapshot {
  path: string;
  mtimeMs: number;
  size: number;
}

export class DaemonConfigManager {
  private manager?: ConnectionManager;
  private snapshot?: ConfigSnapshot;

  async getManager(configPath?: string): Promise<ConnectionManager> {
    const path = configPath || resolveConfigPath();
    const snapshot = await readConfigSnapshot(path);
    if (!this.manager || !this.snapshot || hasConfigChanged(this.snapshot, snapshot)) {
      await this.replaceManager(snapshot);
    }
    return this.manager!;
  }

  status(): { connections: Array<{ name: string; type: string; keepAliveSeconds: number }> } {
    return this.manager?.status() ?? { connections: [] };
  }

  async closeAll(): Promise<void> {
    if (this.manager) {
      await this.manager.closeAll();
      this.manager = undefined;
      this.snapshot = undefined;
    }
  }

  private async replaceManager(snapshot: ConfigSnapshot): Promise<void> {
    if (this.manager) {
      await this.manager.closeAll();
    }
    this.manager = new ConnectionManager(await loadConfig(snapshot.path));
    this.snapshot = snapshot;
  }
}

function hasConfigChanged(current: ConfigSnapshot, next: ConfigSnapshot): boolean {
  return current.path !== next.path || current.mtimeMs !== next.mtimeMs || current.size !== next.size;
}

async function readConfigSnapshot(path: string): Promise<ConfigSnapshot> {
  const stats = await stat(path);
  return {
    path,
    mtimeMs: stats.mtimeMs,
    size: stats.size
  };
}
