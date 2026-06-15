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
  desc "Terminal tabular data explorer — CSV/JSON/Parquet/Excel/SQLite viewer"
  homepage "https://github.com/denisotree/tuitab"
  license "Apache-2.0"
  head "https://github.com/denisotree/tuitab.git", branch: "master"

  on_macos do
    on_arm do
      url "https://github.com/denisotree/tuitab/releases/download/v0.4.3/tuitab-v0.4.3-aarch64-apple-darwin.tar.gz"
      sha256 "dc101e42e46d6a1d6ceaa80a32099e171df735b23a729427bb4178b00598f2aa"
      version "0.4.3"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.4.3/tuitab-v0.4.3-x86_64-apple-darwin.tar.gz"
      sha256 "2a218daa979c346fa33ac4fa8c97b0cea7b0b8b30a5b4b2d23886c9bd5e8e44b"
      version "0.4.3"
    end
  end
  on_linux do
    on_arm do
      url "https://github.com/denisotree/tuitab/releases/download/v0.4.3/tuitab-v0.4.3-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "c67d17a0467a753a0102aa9f7d5d821c4830a084b3de0dd3119f6a3f8a6e14a1"
      version "0.4.3"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.4.3/tuitab-v0.4.3-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "eac651f8807a53682fdfe87fb8e64739761db05f5a69daa4243ec045924c0a24"
      version "0.4.3"
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
