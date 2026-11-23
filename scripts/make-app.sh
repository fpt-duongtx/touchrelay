#!/usr/bin/env bash
# Đóng gói touchrelay thành dist/TouchRelay.app
#
#   ./scripts/make-app.sh                 # build cho chip của máy này
#   UNIVERSAL=1 ./scripts/make-app.sh     # build universal (Apple Silicon + Intel)
#   SIGN_IDENTITY="Apple Development: Ten Ban (XXXXXXXXXX)" ./scripts/make-app.sh
#   INSTALL=1 ./scripts/make-app.sh       # chép luôn vào /Applications
set -euo pipefail
cd "$(dirname "$0")/.."

APP_NAME="TouchRelay"
BUNDLE_ID="${BUNDLE_ID:-local.touchrelay}"
LOCAL_ID="Touchmac Local Signing" # chứng chỉ tự cấp, giữ quyền TCC qua các lần build
if [[ -z "${SIGN_IDENTITY:-}" ]]; then
  if security find-identity -p codesigning 2>/dev/null | grep -q "\"$LOCAL_ID\""; then
    SIGN_IDENTITY="$LOCAL_ID"
  else
    SIGN_IDENTITY="-" # "-" = ký ad-hoc
  fi
fi
VERSION="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
DIST="dist"
APP="$DIST/$APP_NAME.app"

if [[ "$(uname)" != "Darwin" ]]; then
  echo "Script này phải chạy trên macOS." >&2
  exit 1
fi

echo "▶︎ Build touchrelay $VERSION"
if [[ "${UNIVERSAL:-0}" == "1" ]]; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin
  cargo build --release --target aarch64-apple-darwin
  cargo build --release --target x86_64-apple-darwin
  mkdir -p target/universal
  BIN="target/universal/touchrelay"
  lipo -create -output "$BIN" \
    target/aarch64-apple-darwin/release/touchrelay \
    target/x86_64-apple-darwin/release/touchrelay
else
  cargo build --release
  BIN="target/release/touchrelay"
fi

echo "▶︎ Tạo $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/touchrelay"
cp assets/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
sed -e "s/__VERSION__/$VERSION/g" -e "s/__BUNDLE_ID__/$BUNDLE_ID/g" \
  assets/Info.plist > "$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist" >/dev/null

echo "▶︎ Ký ($( [[ "$SIGN_IDENTITY" == "-" ]] && echo ad-hoc || echo "$SIGN_IDENTITY" ))"
if [[ "$SIGN_IDENTITY" == "-" ]]; then
  codesign --force --sign - --identifier "$BUNDLE_ID" "$APP"
elif [[ "$SIGN_IDENTITY" == "$LOCAL_ID" ]]; then
  codesign --force --timestamp=none --sign "$SIGN_IDENTITY" --identifier "$BUNDLE_ID" "$APP"
else
  codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" --identifier "$BUNDLE_ID" "$APP"
fi
codesign --verify --strict "$APP"

( cd "$DIST" && rm -f "$APP_NAME.zip" && ditto -c -k --keepParent "$APP_NAME.app" "$APP_NAME.zip" )

if [[ "${INSTALL:-0}" == "1" ]]; then
  echo "▶︎ Cài vào /Applications"
  osascript -e "tell application \"$APP_NAME\" to quit" >/dev/null 2>&1 || true
  pkill -x touchrelay 2>/dev/null || true
  rm -rf "/Applications/$APP_NAME.app"
  cp -R "$APP" /Applications/
  APP="/Applications/$APP_NAME.app"
fi

echo
echo "✅ Xong: $APP   (bản zip: $DIST/$APP_NAME.zip)"
echo "   Mở thử: open \"$APP\""
if [[ "$SIGN_IDENTITY" == "-" ]]; then
  echo
  echo "ℹ️  Ký ad-hoc: sau MỖI lần build lại, macOS coi đây là app mới."
  echo "   Nếu TouchRelay báo thiếu quyền, vào System Settings → Privacy & Security →"
  echo "   Input Monitoring và Accessibility, xoá TouchRelay cũ (nút −) rồi bật lại."
  echo "   Hoặc chạy: tccutil reset Accessibility $BUNDLE_ID; tccutil reset ListenEvent $BUNDLE_ID"
fi
