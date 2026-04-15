import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { readDir, mkdir, remove, readTextFile, writeTextFile, exists } from '@tauri-apps/plugin-fs';
import { appDataDir, join } from '@tauri-apps/api/path';
import { initBrowserAutomationTables } from './browserAutomationService';

export { initBrowserAutomationTables };

export interface BrowserExtension {
  id: number;
  name: string;
  description: string | null;
  path: string;
  version: string | null;
  author: string | null;
  enabled: boolean;
  is_builtin: boolean;
  tags: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateExtensionRequest {
  name: string;
  description?: string;
  path: string;
  version?: string;
  author?: string;
  enabled?: boolean;
  tags?: string[];
}

export interface UpdateExtensionRequest {
  id: number;
  name?: string;
  description?: string;
  path?: string;
  version?: string;
  author?: string;
  enabled?: boolean;
  tags?: string[];
}

export interface ExtensionFolder {
  name: string;
  path: string;
  hasManifest: boolean;
  manifestInfo?: {
    name?: string;
    version?: string;
    description?: string;
  };
}

export const extensionService = {
  async getExtensions(): Promise<BrowserExtension[]> {
    return await invoke('get_browser_extensions');
  },

  async createExtension(request: CreateExtensionRequest): Promise<BrowserExtension> {
    return await invoke('create_browser_extension', { request });
  },

  async updateExtension(request: UpdateExtensionRequest): Promise<BrowserExtension> {
    return await invoke('update_browser_extension', { request });
  },

  async deleteExtension(id: number): Promise<void> {
    await invoke('delete_browser_extension', { id });
  },

  async toggleExtension(id: number, enabled: boolean): Promise<BrowserExtension> {
    return await invoke('toggle_browser_extension', { id, enabled });
  },

  async scanExtensionFolder(folderPath: string): Promise<ExtensionFolder[]> {
    console.log('[extensionService] scanExtensionFolder:', folderPath);
    const result = await invoke('scan_extension_folder', { folderPath });
    console.log('[extensionService] scanExtensionFolder result:', result);
    return result as ExtensionFolder[];
  },

  async importExtensionFromFolder(folderPath: string, name?: string): Promise<BrowserExtension> {
    return await invoke('import_extension_from_folder', { folderPath, name });
  },

  async selectExtensionFolder(): Promise<string | null> {
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择浏览器插件目录'
    });
    
    if (selected && !Array.isArray(selected)) {
      return selected;
    }
    return null;
  },

  async selectCrxFile(): Promise<string | null> {
    const selected = await open({
      multiple: false,
      filters: [
        { name: 'Chrome Extension', extensions: ['crx'] },
        { name: 'All Files', extensions: ['*'] }
      ],
      title: '选择 Chrome 扩展文件'
    });
    
    if (selected && !Array.isArray(selected)) {
      return selected;
    }
    return null;
  },

  async getExtensionPath(): Promise<string> {
    const dataDir = await appDataDir();
    const extPath = await join(dataDir, 'browser_extensions');
    return extPath;
  },

  async ensureExtensionDir(): Promise<string> {
    const extPath = await this.getExtensionPath();
    try {
      const dirExists = await exists(extPath);
      if (!dirExists) {
        await mkdir(extPath, { recursive: true });
      }
    } catch (e) {
      console.error('Failed to create extension directory:', e);
    }
    return extPath;
  },

  async getEnabledExtensions(): Promise<BrowserExtension[]> {
    const extensions = await this.getExtensions();
    return extensions.filter(ext => ext.enabled);
  },

  async getExtensionPaths(): Promise<string[]> {
    const enabled = await this.getEnabledExtensions();
    return enabled.map(ext => ext.path);
  },

  async updateExtensionName(id: number, name: string): Promise<BrowserExtension> {
    return await this.updateExtension({ id, name });
  },

  async getExtensionIcon(extensionPath: string): Promise<string | null> {
    try {
      // 尝试查找常见的图标文件
      const iconNames = ['icon.png', 'icon128.png', 'icon48.png', 'icon32.png', 'icon16.png', 'logo.png'];
      for (const iconName of iconNames) {
        const iconPath = await join(extensionPath, iconName);
        try {
          const iconExists = await exists(iconPath);
          if (iconExists) {
            return iconPath;
          }
        } catch (e) {
          // 继续尝试下一个
        }
      }
      return null;
    } catch (e) {
      return null;
    }
  }
};
