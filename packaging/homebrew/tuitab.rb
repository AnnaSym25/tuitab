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
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.2/tuitab-v0.9.2-aarch64-apple-darwin.tar.gz"
      sha256 "26bba151dd93511105d9315d287e5d9964285d9e31bf2ca26b6f15238389fefb"
      version "0.9.2"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.2/tuitab-v0.9.2-x86_64-apple-darwin.tar.gz"
      sha256 "7aaf401b56ca1316c17ddac6fc542c8e905a5de54ed8df1d1356f5e20a33b2d6"
      version "0.9.2"
    end
  end
  on_linux do
    on_arm do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.2/tuitab-v0.9.2-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "0414718b80178556295386a8e77a1a4b25cecfa408e6185480fe65ab74ad5efe"
      version "0.9.2"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.2/tuitab-v0.9.2-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "eee4159ebb9d2fb369be8a0b71f668d7c76f9edd0ffc31d9eab2f3ecaa4f4990"
      version "0.9.2"
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
