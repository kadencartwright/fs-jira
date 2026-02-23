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
    if [ -n "${XDG_CONFIG_HOME:-}" ]; then \
      config_dir="${XDG_CONFIG_HOME}/fs-jira"; \
    elif [ -n "${HOME:-}" ]; then \
      config_dir="${HOME}/.config/fs-jira"; \
    else \
      echo "failed to resolve config path: HOME is not set and XDG_CONFIG_HOME is unset" >&2; \
      exit 1; \
    fi; \
    mkdir -p "$config_dir"; \
    config_path="$config_dir/config.toml"; \
    if [ -e "$config_path" ]; then \
      echo "refusing to overwrite existing config: $config_path" >&2; \
      exit 1; \
    fi; \
    cp config.example.toml "$config_path"; \
    echo "created default config: $config_path"
