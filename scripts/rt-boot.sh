#!/usr/bin/env bash
# TASK-022 RT request (2026-11-11): the Human's boot steps as one script.
# Every step is appended to a log in the repository so the AI can check what
# happened after the reboot.
#
#   sudo scripts/rt-boot.sh once     # boot the RT kernel on the next boot only
#   sudo scripts/rt-boot.sh generic  # boot the generic kernel on the next boot only
#   sudo scripts/rt-boot.sh back     # make sure the next boot is the default
#   scripts/rt-boot.sh status        # record the current state (no sudo)
#
# `once` selects the entry by GRUB's menu entry ids, not by its (possibly
# translated) title, and uses grub-reboot: the default stays as it is, so a
# failed RT boot is undone by powering on again. Nothing is changed in
# /etc/default/grub. Each command asks before it reboots.
#
# Since 7.0.0-44 (2026-11-24) GRUB's default entry is the realtime kernel
# (it sorts before generic of the same version), so a generic comparison
# needs `generic` (TASK-042).
#
# Environment: RT_KERNEL (default: the newest /boot/vmlinuz-*-realtime),
# GENERIC_KERNEL (default: the newest /boot/vmlinuz-*-generic),
# RT_BOOT_LOG (default: .cowork/05_verification/perf_runs/rt-boot/boot.log),
# GRUB_CFG (default /boot/grub/grub.cfg), RT_BOOT_DRY=1 (no grub-reboot, no
# reboot; for checking the script).
set -euo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd)
log=${RT_BOOT_LOG:-$repo/.cowork/05_verification/perf_runs/rt-boot/boot.log}
cfg=${GRUB_CFG:-/boot/grub/grub.cfg}
env_file=/boot/grub/grubenv
dry=${RT_BOOT_DRY:-}
owner=${SUDO_USER:-$(id -un)}

mkdir -p "$(dirname "$log")"
say() {
  printf '%s %s\n' "$(date -Iseconds)" "$*" | tee -a "$log"
}
own_log() {
  chown "$owner": "$(dirname "$log")" "$log" 2>/dev/null || true
}
trap own_log EXIT
need_root() {
  if [ -z "$dry" ] && [ "$(id -u)" != 0 ]; then
    echo "run with sudo: sudo $0 $1" >&2
    exit 2
  fi
}
facts() {
  say "user=$owner kernel=$(uname -r) realtime=$(cat /sys/kernel/realtime 2>/dev/null || echo absent) boot_id=$(cat /proc/sys/kernel/random/boot_id)"
  say "cmdline: $(cat /proc/cmdline)"
  say "grubenv: $(grep -v '^#' "$env_file" | tr '\n' ' ')"
}
ask_reboot() {
  if [ -n "$dry" ]; then
    say "dry run: no reboot"
    return
  fi
  read -r -p "지금 재부팅할까요? [y/N] " answer
  if [ "$answer" = y ] || [ "$answer" = Y ]; then
    say "reboot requested"
    own_log
    sync
    # A desktop session can inhibit it; the boot choice above stays set.
    if ! reboot 2>>"$log"; then
      say "reboot refused (see above); the boot choice stays — restart from the desktop menu or run: sudo systemctl reboot -i"
      exit 1
    fi
  else
    say "reboot postponed by the user (run: sudo reboot)"
  fi
}

# Select `kernel` (realtime or generic flavour) for the next boot only.
boot_once() {
  local flavour=$1 kernel=$2
  if [ -z "$kernel" ] || [ ! -e "/boot/vmlinuz-$kernel" ] || [ ! -e "/boot/initrd.img-$kernel" ]; then
    say "FAIL: no installed $flavour kernel image and initrd (${kernel:-none})"
    exit 1
  fi
  if [ "$flavour" = realtime ]; then
    grep -q '^CONFIG_PREEMPT_RT=y' "/boot/config-$kernel" || {
      say "FAIL: /boot/config-$kernel lacks CONFIG_PREEMPT_RT=y"
      exit 1
    }
  elif grep -q '^CONFIG_PREEMPT_RT=y' "/boot/config-$kernel"; then
    say "FAIL: /boot/config-$kernel has CONFIG_PREEMPT_RT=y; not a generic kernel"
    exit 1
  fi
  # grub-reboot works only if the generated config honours next_entry.
  if ! grep -q 'if \[ "\${next_entry}" \]' "$cfg"; then
    say "FAIL: $cfg does not handle next_entry; grub-reboot would be ignored"
    exit 1
  fi
  # submenu id > menuentry id; the recovery entry has its own id.
  submenu=$(grep -E "^submenu " "$cfg" | grep -o "\\\$menuentry_id_option '[^']*'" | head -1 | cut -d"'" -f2)
  entry=$(grep -E "^\s+menuentry " "$cfg" | grep -o "\\\$menuentry_id_option 'gnulinux-$kernel-advanced-[^']*'" | head -1 | cut -d"'" -f2)
  if [ -z "$submenu" ] || [ -z "$entry" ]; then
    say "FAIL: GRUB entry for $kernel not found in $cfg (submenu='$submenu' entry='$entry')"
    exit 1
  fi
  target="$submenu>$entry"
  say "kernel=$kernel target=$target"
  if [ -n "$dry" ]; then
    say "dry run: grub-reboot skipped"
  else
    grub-reboot "$target"
    next=$(grub-editenv "$env_file" list 2>/dev/null | sed -n 's/^next_entry=//p')
    if [ "$next" != "$target" ]; then
      say "FAIL: next_entry is '$next', expected '$target'"
      exit 1
    fi
    say "OK: next boot only -> $target (default unchanged)"
  fi
  ask_reboot
}

case "${1:-}" in
once)
  need_root once
  say "=== once"
  facts
  boot_once realtime "${RT_KERNEL:-$(ls /boot/vmlinuz-*-realtime 2>/dev/null | sed 's|/boot/vmlinuz-||' | sort -V | tail -1)}"
  ;;
generic)
  need_root generic
  say "=== generic"
  facts
  boot_once generic "${GENERIC_KERNEL:-$(ls /boot/vmlinuz-*-generic 2>/dev/null | sed 's|/boot/vmlinuz-||' | sort -V | tail -1)}"
  ;;
back)
  need_root back
  say "=== back"
  facts
  next=$(grub-editenv "$env_file" list 2>/dev/null | sed -n 's/^next_entry=//p')
  if [ -n "$next" ]; then
    if [ -n "$dry" ]; then
      say "dry run: would clear next_entry=$next"
    else
      grub-editenv "$env_file" unset next_entry
      say "cleared leftover next_entry=$next"
    fi
  fi
  say "OK: next boot uses the default entry"
  ask_reboot
  ;;
status)
  say "=== status"
  facts
  ;;
*)
  sed -n '2,20p' "$0"
  exit 2
  ;;
esac
