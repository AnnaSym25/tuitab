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
      url "https://github.com/denisotree/tuitab/releases/download/v0.5.0/tuitab-v0.5.0-aarch64-apple-darwin.tar.gz"
      sha256 "ead8362504fff258c5716a9caa97a7b73c050b86a9f97a1a039f63c94472c442"
      version "0.5.0"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.5.0/tuitab-v0.5.0-x86_64-apple-darwin.tar.gz"
      sha256 "fcbf69e3aebc3eaa242dce2a358b40ad8acfa53a74f6bd766caf366c82a091f6"
      version "0.5.0"
    end
  end
  on_linux do
    on_arm do
      url "https://github.com/denisotree/tuitab/releases/download/v0.5.0/tuitab-v0.5.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "7d900dd25a463df71936796b4c2a42404ae001296ef9cb9539a578a3fef1cec5"
      version "0.5.0"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.5.0/tuitab-v0.5.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "9746a161d3d305ef2becab6073c28182ea88252b2881d5720165bfb67ad26b09"
      version "0.5.0"
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
