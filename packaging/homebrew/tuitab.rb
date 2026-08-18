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
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.4/tuitab-v0.9.4-aarch64-apple-darwin.tar.gz"
      sha256 "56846b6b6f46920b8514c6fdc95e2cbd2e42d477a4be1a169c3c0696557f8d8d"
      version "0.9.4"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.4/tuitab-v0.9.4-x86_64-apple-darwin.tar.gz"
      sha256 "dd88882c244d729b9a100ef9e570927c163804fe34651527dd4bd3971d1c19ce"
      version "0.9.4"
    end
  end
  on_linux do
    on_arm do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.4/tuitab-v0.9.4-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "7ae2583ad6d2176b012ffd48535be99a8860a41a20e0c0cf31122d946c6688a0"
      version "0.9.4"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.4/tuitab-v0.9.4-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "030e8b1a6e186f01e49a934754932e705daa359cfaaf0d776d257f4d00832a67"
      version "0.9.4"
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
