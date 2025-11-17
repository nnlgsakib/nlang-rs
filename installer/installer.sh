#!/usr/bin/env bash
set -e

if [ "$(uname -s)" != "Linux" ]; then
  echo "This installer supports Linux only"
  exit 1
fi

if command -v gcc >/dev/null 2>&1; then
  ver=$(gcc -dumpfullversion -dumpversion 2>/dev/null || true)
  major=${ver%%.*}
  if [ "$major" != "10" ]; then
    if command -v apt-get >/dev/null 2>&1; then
      sudo apt-get update -y
      sudo apt-get install -y gcc-10 g++-10
      sudo update-alternatives --install /usr/bin/gcc gcc /usr/bin/gcc-10 100
      sudo update-alternatives --install /usr/bin/g++ g++ /usr/bin/g++-10 100
    else
      echo "gcc found but not 10.x; please install gcc-10 manually"
      exit 1
    fi
  fi
else
  if command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update -y
    sudo apt-get install -y gcc-10 g++-10
    sudo update-alternatives --install /usr/bin/gcc gcc /usr/bin/gcc-10 100
    sudo update-alternatives --install /usr/bin/g++ g++ /usr/bin/g++-10 100
  else
    echo "gcc not found and apt-get unavailable; please install gcc 10.x manually"
    exit 1
  fi
fi

INSTALL_DIR="$HOME/.nlang"
mkdir -p "$INSTALL_DIR"

NLANG_URL="https://github.com/nnlgsakib/nlang/releases/download/2.0.1/nlang"
NSCAN_URL="https://github.com/nnlgsakib/nlang/releases/download/2.0.1/nscan"

download() {
  url="$1"
  out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$out"
  elif command -v wget >/dev/null 2>&1; then
    wget -q "$url" -O "$out"
  else
    if command -v apt-get >/dev/null 2>&1; then
      sudo apt-get update -y
      sudo apt-get install -y curl
      curl -fsSL "$url" -o "$out"
    else
      echo "Neither curl nor wget found; please install one"
      exit 1
    fi
  fi
}

download "$NLANG_URL" "$INSTALL_DIR/nlang"
download "$NSCAN_URL" "$INSTALL_DIR/nscan"

chmod +x "$INSTALL_DIR/nlang" "$INSTALL_DIR/nscan"

SHELL_NAME="$(basename "$SHELL")"
if [ "$SHELL_NAME" = "zsh" ]; then
  RC_FILE="$HOME/.zshrc"
else
  RC_FILE="$HOME/.bashrc"
fi

if [ ! -f "$RC_FILE" ]; then
  touch "$RC_FILE"
fi

if ! grep -q "\.nlang" "$RC_FILE"; then
  echo "export PATH=\"$HOME/.nlang:\$PATH\"" >> "$RC_FILE"
fi

echo "Installed to $INSTALL_DIR"
echo "Added PATH update to $RC_FILE"
echo "Run: source $RC_FILE"