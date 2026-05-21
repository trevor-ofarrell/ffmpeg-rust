#!/usr/bin/env bash
set -euo pipefail

FFMPEG_TAG="${FFMPEG_TAG:-n8.1.1}"
FFMPEG_VERSION="${FFMPEG_VERSION:-${FFMPEG_TAG#n}}"
WSL_DISTRO="${WSL_DISTRO_NAME:-Ubuntu}"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

work_root="${FFMPEGRUST_ORACLE_WORK:-$HOME/.cache/ffmpegrust/ffmpeg-oracle-$FFMPEG_TAG}"
src_dir="$work_root/src"
build_dir="$work_root/build"
install_dir="$repo_root/third_party/ffmpeg-oracle/wsl"
wrapper_dir="$repo_root/third_party/ffmpeg-oracle/build/bin"

mkdir -p "$work_root" "$wrapper_dir"

if [[ ! -d "$src_dir/.git" ]]; then
  git clone --depth 1 --branch "$FFMPEG_TAG" https://git.ffmpeg.org/ffmpeg.git "$src_dir"
fi

# Git tag builds report versions like "n8.1.1". The release tarball includes
# VERSION, so write it here to keep the local oracle banner release-shaped.
printf '%s\n' "$FFMPEG_VERSION" > "$src_dir/VERSION"

mkdir -p "$build_dir"
cd "$build_dir"

"$src_dir/configure" \
  --prefix="$install_dir" \
  --disable-gpl \
  --disable-nonfree \
  --disable-doc \
  --disable-x86asm

make -j"$(nproc)"
make install

runner_path="$wrapper_dir/oracle-wsl-runner.sh"
cat > "$runner_path" <<'RUNNER'
#!/usr/bin/env bash
set -euo pipefail

tool="${1:?missing ffmpeg tool name}"
shift

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bin_dir="$(cd "$script_dir/../../wsl/bin" && pwd)"

converted_args=()
for arg in "$@"; do
  if [[ "$arg" =~ ^[A-Za-z]:[\\/] ]]; then
    converted_args+=("$(wslpath -u "$arg")")
  else
    converted_args+=("$arg")
  fi
done

exec "$bin_dir/$tool" "${converted_args[@]}"
RUNNER
chmod +x "$runner_path"

cat > "$wrapper_dir/ffmpeg" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$script_dir/../../wsl/bin/ffmpeg" "$@"
SH
chmod +x "$wrapper_dir/ffmpeg"

cat > "$wrapper_dir/ffprobe" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$script_dir/../../wsl/bin/ffprobe" "$@"
SH
chmod +x "$wrapper_dir/ffprobe"

runner_wsl_path="$runner_path"
cat > "$wrapper_dir/ffmpeg.cmd" <<CMD
@echo off
wsl.exe -d $WSL_DISTRO --exec bash "$runner_wsl_path" ffmpeg %*
CMD

cat > "$wrapper_dir/ffprobe.cmd" <<CMD
@echo off
wsl.exe -d $WSL_DISTRO --exec bash "$runner_wsl_path" ffprobe %*
CMD

"$install_dir/bin/ffmpeg" -version | head -1
"$install_dir/bin/ffprobe" -version | head -1
echo "Installed WSL oracle wrappers under $wrapper_dir"
