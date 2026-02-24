set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

build:
    cargo build --locked

run mountpoint="/tmp/fs-jira-mnt":
    mkdir -p "{{mountpoint}}"
    cargo run --locked -- "{{mountpoint}}"

run-with-config config_path mountpoint="/tmp/fs-jira-mnt":
    mkdir -p "{{mountpoint}}"
    cargo run --locked -- --config "{{config_path}}" "{{mountpoint}}"

service-install mountpoint="" config_path="":
    bin_path="$(command -v fs-jira || true)"; \
    if [ -z "$bin_path" ]; then \
      echo "fs-jira binary not found on PATH; run just install first" >&2; \
      exit 1; \
    fi; \
    if [ -n "{{config_path}}" ]; then \
      resolved_config="{{config_path}}"; \
    elif [ -n "${XDG_CONFIG_HOME:-}" ]; then \
      resolved_config="${XDG_CONFIG_HOME}/fs-jira/config.toml"; \
    elif [ -n "${HOME:-}" ]; then \
      resolved_config="${HOME}/.config/fs-jira/config.toml"; \
    else \
      echo "failed to resolve config path: HOME is not set and XDG_CONFIG_HOME is unset" >&2; \
      exit 1; \
    fi; \
    if [ ! -f "$resolved_config" ]; then \
      echo "config file not found at $resolved_config" >&2; \
      exit 1; \
    fi; \
    if [ -n "{{mountpoint}}" ]; then \
      mountpoint_input="{{mountpoint}}"; \
    else \
      mountpoint_input="~/fs-jira"; \
    fi; \
    case "$mountpoint_input" in \
      "~") resolved_mountpoint="${HOME}" ;; \
      "~/"*) resolved_mountpoint="${HOME}/${mountpoint_input#\~/}" ;; \
      *) resolved_mountpoint="$mountpoint_input" ;; \
    esac; \
    mkdir -p "$resolved_mountpoint"; \
    os_name="$(uname -s)"; \
    if [ "$os_name" = "Linux" ]; then \
      target_dir="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"; \
      target_path="$target_dir/fs-jira.service"; \
      template_path="deploy/systemd/fs-jira.service.tmpl"; \
      mkdir -p "$target_dir"; \
      BIN_PATH="$bin_path" CONFIG_PATH="$resolved_config" MOUNTPOINT="$resolved_mountpoint" HOME_DIR="$HOME" TARGET_PATH="$target_path" TEMPLATE_PATH="$template_path" python -c 'import os,pathlib; t=pathlib.Path(os.environ["TEMPLATE_PATH"]).read_text(); t=t.replace("__BIN_PATH__",os.environ["BIN_PATH"]).replace("__CONFIG_PATH__",os.environ["CONFIG_PATH"]).replace("__MOUNTPOINT__",os.environ["MOUNTPOINT"]).replace("__HOME_DIR__",os.environ["HOME_DIR"]); pathlib.Path(os.environ["TARGET_PATH"]).write_text(t)'; \
      echo "installed systemd user service: $target_path"; \
    elif [ "$os_name" = "Darwin" ]; then \
      target_dir="$HOME/Library/LaunchAgents"; \
      target_path="$target_dir/com.fs-jira.mount.plist"; \
      template_path="deploy/launchd/com.fs-jira.mount.plist.tmpl"; \
      mkdir -p "$target_dir" "$HOME/Library/Logs"; \
      BIN_PATH="$bin_path" CONFIG_PATH="$resolved_config" MOUNTPOINT="$resolved_mountpoint" HOME_DIR="$HOME" TARGET_PATH="$target_path" TEMPLATE_PATH="$template_path" python -c 'import os,pathlib; t=pathlib.Path(os.environ["TEMPLATE_PATH"]).read_text(); t=t.replace("__BIN_PATH__",os.environ["BIN_PATH"]).replace("__CONFIG_PATH__",os.environ["CONFIG_PATH"]).replace("__MOUNTPOINT__",os.environ["MOUNTPOINT"]).replace("__HOME_DIR__",os.environ["HOME_DIR"]); pathlib.Path(os.environ["TARGET_PATH"]).write_text(t)'; \
      echo "installed launchd agent: $target_path"; \
    else \
      echo "unsupported OS for service-install: $os_name" >&2; \
      exit 1; \
    fi

service-enable:
    os_name="$(uname -s)"; \
    if [ "$os_name" = "Linux" ]; then \
      systemctl --user daemon-reload; \
      systemctl --user enable --now fs-jira.service; \
    elif [ "$os_name" = "Darwin" ]; then \
      plist_path="$HOME/Library/LaunchAgents/com.fs-jira.mount.plist"; \
      launchctl bootout "gui/$(id -u)" "$plist_path" >/dev/null 2>&1 || true; \
      launchctl bootstrap "gui/$(id -u)" "$plist_path"; \
      launchctl kickstart -k "gui/$(id -u)/com.fs-jira.mount"; \
    else \
      echo "unsupported OS for service-enable: $os_name" >&2; \
      exit 1; \
    fi

service-start:
    os_name="$(uname -s)"; \
    if [ "$os_name" = "Linux" ]; then \
      systemctl --user start fs-jira.service; \
    elif [ "$os_name" = "Darwin" ]; then \
      launchctl kickstart -k "gui/$(id -u)/com.fs-jira.mount"; \
    else \
      echo "unsupported OS for service-start: $os_name" >&2; \
      exit 1; \
    fi

service-stop:
    os_name="$(uname -s)"; \
    if [ "$os_name" = "Linux" ]; then \
      systemctl --user stop fs-jira.service; \
    elif [ "$os_name" = "Darwin" ]; then \
      launchctl bootout "gui/$(id -u)/com.fs-jira.mount" >/dev/null 2>&1 || true; \
    else \
      echo "unsupported OS for service-stop: $os_name" >&2; \
      exit 1; \
    fi

service-disable:
    os_name="$(uname -s)"; \
    if [ "$os_name" = "Linux" ]; then \
      systemctl --user disable --now fs-jira.service; \
    elif [ "$os_name" = "Darwin" ]; then \
      launchctl bootout "gui/$(id -u)/com.fs-jira.mount" >/dev/null 2>&1 || true; \
    else \
      echo "unsupported OS for service-disable: $os_name" >&2; \
      exit 1; \
    fi

service-status:
    os_name="$(uname -s)"; \
    if [ "$os_name" = "Linux" ]; then \
      systemctl --user status fs-jira.service --no-pager; \
    elif [ "$os_name" = "Darwin" ]; then \
      launchctl print "gui/$(id -u)/com.fs-jira.mount"; \
    else \
      echo "unsupported OS for service-status: $os_name" >&2; \
      exit 1; \
    fi

service-logs:
    os_name="$(uname -s)"; \
    if [ "$os_name" = "Linux" ]; then \
      journalctl --user -u fs-jira.service --no-pager -n 100; \
    elif [ "$os_name" = "Darwin" ]; then \
      echo "--- $HOME/Library/Logs/fs-jira.log ---"; \
      tail -n 100 "$HOME/Library/Logs/fs-jira.log" 2>/dev/null || true; \
      echo "--- $HOME/Library/Logs/fs-jira.err.log ---"; \
      tail -n 100 "$HOME/Library/Logs/fs-jira.err.log" 2>/dev/null || true; \
    else \
      echo "unsupported OS for service-logs: $os_name" >&2; \
      exit 1; \
    fi

service-uninstall:
    os_name="$(uname -s)"; \
    if [ "$os_name" = "Linux" ]; then \
      target_path="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/fs-jira.service"; \
      systemctl --user disable --now fs-jira.service >/dev/null 2>&1 || true; \
      rm -f "$target_path"; \
      systemctl --user daemon-reload; \
      echo "removed systemd user service: $target_path"; \
    elif [ "$os_name" = "Darwin" ]; then \
      target_path="$HOME/Library/LaunchAgents/com.fs-jira.mount.plist"; \
      launchctl bootout "gui/$(id -u)" "$target_path" >/dev/null 2>&1 || true; \
      rm -f "$target_path"; \
      echo "removed launchd agent: $target_path"; \
    else \
      echo "unsupported OS for service-uninstall: $os_name" >&2; \
      exit 1; \
    fi

install:
    cargo install --path . --locked
    if [ -z "${HOME:-}" ]; then \
      echo "HOME is not set" >&2; \
      exit 1; \
    fi
    config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/fs-jira"; \
    mkdir -p "$config_dir"; \
    config_path="$config_dir/config.toml"; \
    if [ -e "$config_path" ]; then \
      echo "config already exists, skipping bootstrap: $config_path"; \
    fi
    if ! command -v npm >/dev/null 2>&1; then \
      echo "npm is required to install the desktop app; install Node.js and retry" >&2; \
      exit 1; \
    fi
    npm --prefix apps/desktop ci
    npm --prefix apps/desktop run tauri:build -- --no-bundle
    desktop_bin="apps/desktop/src-tauri/target/release/fs-jira-desktop"; \
    if [ ! -x "$desktop_bin" ]; then \
      echo "desktop binary not found at $desktop_bin" >&2; \
      exit 1; \
    fi; \
    local_bin_dir="$HOME/.local/bin"; \
    mkdir -p "$local_bin_dir"; \
    cp "$desktop_bin" "$local_bin_dir/fs-jira-desktop"; \
    chmod +x "$local_bin_dir/fs-jira-desktop"; \
    os_name="$(uname -s)"; \
    if [ "$os_name" = "Linux" ]; then \
      data_home="${XDG_DATA_HOME:-$HOME/.local/share}"; \
      icon_dir="$data_home/icons/hicolor/256x256/apps"; \
      desktop_dir="$data_home/applications"; \
      desktop_file="$desktop_dir/fs-jira-desktop.desktop"; \
      legacy_desktop_file="$desktop_dir/com.fs-jira.desktop.desktop"; \
      mkdir -p "$icon_dir" "$desktop_dir"; \
      rm -f "$legacy_desktop_file"; \
      cp apps/desktop/src-tauri/icons/icon.png "$icon_dir/fs-jira-desktop.png"; \
      DESKTOP_FILE="$desktop_file" DESKTOP_BIN="$local_bin_dir/fs-jira-desktop" python -c 'import os,pathlib; p=pathlib.Path(os.environ["DESKTOP_FILE"]); p.write_text("[Desktop Entry]\nType=Application\nName=fs-jira Desktop\nComment=fs-jira service control panel\nExec=\"{}\"\nIcon=fs-jira-desktop\nTerminal=false\nCategories=Development;Utility;\nStartupNotify=true\n".format(os.environ["DESKTOP_BIN"]))'; \
      chmod 644 "$desktop_file"; \
      update-desktop-database "$desktop_dir" >/dev/null 2>&1 || true; \
      echo "installed desktop entry: $desktop_file"; \
    elif [ "$os_name" = "Darwin" ]; then \
      app_dir="$HOME/Applications/fs-jira Desktop.app"; \
      contents_dir="$app_dir/Contents"; \
      macos_dir="$contents_dir/MacOS"; \
      resources_dir="$contents_dir/Resources"; \
      info_plist="$contents_dir/Info.plist"; \
      launcher_script="$macos_dir/fs-jira-desktop"; \
      mkdir -p "$macos_dir" "$resources_dir"; \
      cp "$local_bin_dir/fs-jira-desktop" "$resources_dir/fs-jira-desktop-bin"; \
      cp apps/desktop/src-tauri/icons/icon.png "$resources_dir/icon.png"; \
      LAUNCHER_SCRIPT="$launcher_script" RESOURCES_DIR="$resources_dir" python -c 'import os,pathlib; p=pathlib.Path(os.environ["LAUNCHER_SCRIPT"]); p.write_text("#!/bin/bash\nexec \\\"{}/fs-jira-desktop-bin\\\" \\\"$@\\\"\n".format(os.environ["RESOURCES_DIR"])); p.chmod(0o755)'; \
      INFO_PLIST="$info_plist" python -c 'import os,pathlib; p=pathlib.Path(os.environ["INFO_PLIST"]); p.write_text("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>CFBundleDisplayName</key>\n  <string>fs-jira Desktop</string>\n  <key>CFBundleExecutable</key>\n  <string>fs-jira-desktop</string>\n  <key>CFBundleIdentifier</key>\n  <string>com.fs-jira.desktop</string>\n  <key>CFBundleName</key>\n  <string>fs-jira Desktop</string>\n  <key>CFBundlePackageType</key>\n  <string>APPL</string>\n  <key>CFBundleShortVersionString</key>\n  <string>0.1.0</string>\n  <key>CFBundleVersion</key>\n  <string>0.1.0</string>\n  <key>LSMinimumSystemVersion</key>\n  <string>12.0</string>\n</dict>\n</plist>\n")'; \
      echo "installed macOS app bundle: $app_dir"; \
    else \
      echo "desktop app launcher setup skipped (unsupported OS: $os_name)"; \
    fi; \
    config_path="${XDG_CONFIG_HOME:-$HOME/.config}/fs-jira/config.toml"; \
    if [ ! -e "$config_path" ]; then \
      cp config.example.toml "$config_path"; \
      echo "created default config: $config_path"; \
    fi
