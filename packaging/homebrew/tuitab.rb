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
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.0/tuitab-v0.9.0-aarch64-apple-darwin.tar.gz"
      sha256 "7873decd07702d8bcd3cd6804aed308b6b2f2a61453b7ff5d4f62fecc865e9cb"
      version "0.9.0"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.0/tuitab-v0.9.0-x86_64-apple-darwin.tar.gz"
      sha256 "0cf0f3fecdb92b918766eb5d2a19265d6878718a635be2c07e67b6d1754eeecc"
      version "0.9.0"
    end
  end
  on_linux do
    on_arm do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.0/tuitab-v0.9.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "23796ab9a45469a94f919ee76d0df8af89afd293a1ed5c740d3b94cfa7dcc0a1"
      version "0.9.0"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.0/tuitab-v0.9.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "e23dcc9b9147e10fce4c772bd10ea8e9173752f9e1d6dfd5909397c67a47dcc8"
      version "0.9.0"
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
