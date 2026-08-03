# Homebrew formula for tuitab.
#
# To publish this formula, create a tap repository:
#   https://github.com/denisotree/homebrew-tuitab
# and place this file at Formula/tuitab.rb inside that repo.
#
# Users install via:
#   brew tap denisotree/tuitab
#   brew install tuitab
#
# Before a release: replace sha256 values with the real checksums from
# `shasum -a 256 <tarball>` for each GitHub Release asset.

class Tuitab < Formula
  desc "Terminal tabular data explorer — CSV/JSON/YAML/TOML/Parquet/Excel/SQLite viewer"
  homepage "https://github.com/denisotree/tuitab"
  license "Apache-2.0"
  head "https://github.com/denisotree/tuitab.git", branch: "master"

  on_macos do
    on_arm do
      url "https://github.com/denisotree/tuitab/releases/download/v0.6.0/tuitab-v0.6.0-aarch64-apple-darwin.tar.gz"
      sha256 "c1c1df5a6b30ac5db4d2e62d9cf5ac3f91b0316789b6f7932e2c06ae99745096"
      version "0.6.0"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.6.0/tuitab-v0.6.0-x86_64-apple-darwin.tar.gz"
      sha256 "ad0ddd98519e2b9341c6961fdaff3ba1e3279e745cf27174036b2953910e7e04"
      version "0.6.0"
    end
  end
  on_linux do
    on_arm do
      url "https://github.com/denisotree/tuitab/releases/download/v0.6.0/tuitab-v0.6.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "6207a81a184cd0e1e2b1c6b0e9daefc4bf39d09ca8b1ee11704148e7d95e0fdf"
      version "0.6.0"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.6.0/tuitab-v0.6.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "17797e89b8105a261bf196355a1b07ecd983455dff46d24de55c093ed38a5f94"
      version "0.6.0"
    end
  end

  def install
    bin.install "tuitab"
    bin.install_symlink bin/"tuitab" => "ttab"
    bin.install_symlink bin/"tuitab" => "ttb"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tuitab --version")
    assert_match version.to_s, shell_output("#{bin}/ttab --version")
    assert_match version.to_s, shell_output("#{bin}/ttb --version")
  end
end
