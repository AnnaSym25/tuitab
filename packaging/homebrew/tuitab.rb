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
      url "https://github.com/denisotree/tuitab/releases/download/v0.8.2/tuitab-v0.8.2-aarch64-apple-darwin.tar.gz"
      sha256 "9048746548ffeabc5d4ba750ec70c6d93b319f1dec45a1743165d63123f268ba"
      version "0.8.2"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.8.2/tuitab-v0.8.2-x86_64-apple-darwin.tar.gz"
      sha256 "baf06593c62d0fc38fb98d8205d7f3b866ac25802cb6fb7c662b858b1b38e838"
      version "0.8.2"
    end
  end
  on_linux do
    on_arm do
      url "https://github.com/denisotree/tuitab/releases/download/v0.8.2/tuitab-v0.8.2-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "097f5bb726fb106f8a6a2dab450d73bebe03c6511041f3cb3f165f261d90fd32"
      version "0.8.2"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.8.2/tuitab-v0.8.2-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "e34e8ff2d07895d40e5a76d92b1ed24ab7eecb58f4289a02df57472f7955d546"
      version "0.8.2"
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
