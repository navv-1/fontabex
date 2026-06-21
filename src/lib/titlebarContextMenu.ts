import { invoke } from "@tauri-apps/api/core";
import { Menu } from "@tauri-apps/api/menu";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

export async function showTitlebarContextMenu(event: MouseEvent) {
  const target = event.target as HTMLElement | null;
  if (
    target?.closest(
      "button, a, input, select, textarea, .toolbar-actions, .window-controls",
    )
  ) {
    return;
  }

  event.preventDefault();

  try {
    const didShowNativeMenu = await invoke<boolean>("show_native_window_menu");
    if (didShowNativeMenu) return;
  } catch (error) {
    console.warn("Failed to show native titlebar menu", error);
  }

  try {
    const isMaximized = await appWindow.isMaximized();
    const menu = await Menu.new({
      items: [
        {
          id: "restore",
          text: "Restore",
          enabled: isMaximized,
          action: () => {
            void appWindow.unmaximize();
          },
        },
        {
          id: "move",
          text: "Move",
          enabled: !isMaximized,
          action: () => {
            void appWindow.startDragging();
          },
        },
        {
          id: "size",
          text: "Size",
          enabled: !isMaximized,
          action: () => {
            void appWindow.startResizeDragging("SouthEast");
          },
        },
        {
          id: "minimize",
          text: "Minimize",
          action: () => {
            void appWindow.minimize();
          },
        },
        {
          id: "maximize",
          text: "Maximize",
          enabled: !isMaximized,
          action: () => {
            void appWindow.maximize();
          },
        },
        { item: "Separator" },
        {
          id: "close",
          text: "Close",
          accelerator: "Alt+F4",
          action: () => {
            void appWindow.close();
          },
        },
      ],
    });

    try {
      await menu.popup(undefined, appWindow);
    } finally {
      void menu.close();
    }
  } catch (error) {
    console.warn("Failed to show titlebar context menu", error);
  }
}
