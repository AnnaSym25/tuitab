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
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.5/tuitab-v0.9.5-aarch64-apple-darwin.tar.gz"
      sha256 "8952111c754a3b5792f81cbbb9da0b77aca81086f291c865068e24c6dec2923c"
      version "0.9.5"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.5/tuitab-v0.9.5-x86_64-apple-darwin.tar.gz"
      sha256 "c93423fac6aa141a42d268235cf8ada3314ad34b10225697403e099ca936549e"
      version "0.9.5"
    end
  end
  on_linux do
    on_arm do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.5/tuitab-v0.9.5-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "e1d24acc5362579a6f1b761132e17def1a3a743ecfc1fe533c427758ecd94e1f"
      version "0.9.5"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.5/tuitab-v0.9.5-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "caac83bb357d717fa2eedc52ab000b390b0b99a6232a266edf95b3767bc06509"
      version "0.9.5"
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
