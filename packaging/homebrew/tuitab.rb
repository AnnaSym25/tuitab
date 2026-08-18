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
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.3/tuitab-v0.9.3-aarch64-apple-darwin.tar.gz"
      sha256 "2ecb4a696144625e5fd1311500253e1acd5b349a5acc40102ab438de337ea5ab"
      version "0.9.3"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.3/tuitab-v0.9.3-x86_64-apple-darwin.tar.gz"
      sha256 "8b85f71936cb8f97173e927e4a5f166ba2757a11871e49e33e4f0b6587eed295"
      version "0.9.3"
    end
  end
  on_linux do
    on_arm do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.3/tuitab-v0.9.3-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "516ed1d0b270d8ef880bbb03f7606b117bcc3f2e60b8da1b505c70d4612150c6"
      version "0.9.3"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.3/tuitab-v0.9.3-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "f7e33b6dfdf848b393fd20a7ebfef9d094f9151dfddbeab29c70ab9d927efcbf"
      version "0.9.3"
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
