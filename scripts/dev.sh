#!/usr/bin/env bash
# Vòng dev nhanh: build bản debug-log, chạy trong N giây, ghi log ra dev.log trong project.
#
#   ./scripts/dev.sh            # chạy 20 giây, chạm thử màn hình trong lúc đó
#   ./scripts/dev.sh 60         # chạy 60 giây
#   ./scripts/dev.sh 20 --one-finger-scroll   # thêm cờ tuỳ ý
#
# Lần đầu: cấp quyền Input Monitoring + Accessibility cho Terminal (một lần, không mất khi build lại).
set -uo pipefail
cd "$(dirname "$0")/.."
[[ -f "$HOME/.cargo/env" ]] && . "$HOME/.cargo/env"

SECS="${1:-20}"
shift || true
LOG="dev.log"

# TouchRelay.app đang chạy sẽ giữ độc quyền màn hình cảm ứng -> tắt trước.
pkill -x touchrelay 2>/dev/null && sleep 0.5

cargo build --release 2>&1 | tail -5 || exit 1

{
  echo "=== $(date '+%F %T')  macOS $(sw_vers -productVersion)  $(uname -m)"
  echo "--- touchrelay --list"
  ./target/release/touchrelay --list
  echo
  echo "--- touchrelay --debug $*  (${SECS}s)"
} 2>&1 | tee "$LOG"

echo
echo "👉 Hãy CHẠM / KÉO / dùng 2 ngón trên màn hình cảm ứng trong ${SECS} giây tới..."
./target/release/touchrelay --debug "$@" 2>&1 | tee -a "$LOG" &
sleep "$SECS"
pkill -INT -x touchrelay 2>/dev/null
wait 2>/dev/null

LINES=$(wc -l < "$LOG" | tr -d ' ')
echo
echo "✅ Đã ghi $LINES dòng vào $(pwd)/$LOG"
# Copy phần quan trọng vào clipboard để dán vào chat (tối đa 400 dòng đầu).
head -n 400 "$LOG" | pbcopy && echo "📋 Đã copy log vào clipboard — dán (⌘V) vào chat với Claude."
