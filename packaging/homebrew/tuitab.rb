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
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.1/tuitab-v0.9.1-aarch64-apple-darwin.tar.gz"
      sha256 "885531ac38f61d9a8844a97daf061c77b7c93192b67365990c1256f71be2a5be"
      version "0.9.1"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.1/tuitab-v0.9.1-x86_64-apple-darwin.tar.gz"
      sha256 "bd5cd58286aee33943a299f123a8ab9efaa3c1e14cfcb0ab6ffded650b8a1064"
      version "0.9.1"
    end
  end
  on_linux do
    on_arm do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.1/tuitab-v0.9.1-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "3a8a51007d5ec09a7b2a315456c4b3225b92871de5786ba32d1ed49712ae0023"
      version "0.9.1"
    end
    on_intel do
      url "https://github.com/denisotree/tuitab/releases/download/v0.9.1/tuitab-v0.9.1-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "84916c35e18d5ced64a451faa4adb7e1a3cf9f5f731ea4d332809faed3286b63"
      version "0.9.1"
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
