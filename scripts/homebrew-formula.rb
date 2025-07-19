# Homebrew Formula for S3 Vectors CLI
# 
# To install:
# 1. Create a tap: brew tap USER/s3-vectors
# 2. Add this formula to the tap repository
# 3. Users can then: brew install s3-vectors
#
# Or for testing locally:
# brew install --build-from-source ./homebrew-formula.rb

class S3Vectors < Formula
  desc "AWS S3 Vectors CLI - Manage vector storage and similarity search"
  homepage "https://github.com/USER/s3-vectors-rust"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/USER/s3-vectors-rust/releases/download/v#{version}/s3-vectors-darwin-x86_64"
      sha256 "PLACEHOLDER_SHA256_DARWIN_X86_64"
    else
      url "https://github.com/USER/s3-vectors-rust/releases/download/v#{version}/s3-vectors-darwin-aarch64"
      sha256 "PLACEHOLDER_SHA256_DARWIN_AARCH64"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      if Hardware::CPU.is_64_bit?
        url "https://github.com/USER/s3-vectors-rust/releases/download/v#{version}/s3-vectors-linux-x86_64"
        sha256 "PLACEHOLDER_SHA256_LINUX_X86_64"
      end
    else
      url "https://github.com/USER/s3-vectors-rust/releases/download/v#{version}/s3-vectors-linux-aarch64"
      sha256 "PLACEHOLDER_SHA256_LINUX_AARCH64"
    end
  end

  def install
    bin.install "s3-vectors-#{OS.mac? ? "darwin" : "linux"}-#{Hardware::CPU.intel? ? "x86_64" : "aarch64"}" => "s3-vectors"
  end

  def caveats
    <<~EOS
      S3 Vectors CLI has been installed!
      
      To get started, run:
        s3-vectors init
      
      This will help you configure your AWS credentials.
      
      For more information:
        s3-vectors --help
        
      Documentation: https://github.com/USER/s3-vectors-rust
    EOS
  end

  test do
    # Test that the binary runs and shows version
    assert_match "s3-vectors", shell_output("#{bin}/s3-vectors --version")
    
    # Test that help command works
    assert_match "AWS S3 Vectors CLI", shell_output("#{bin}/s3-vectors --help")
  end
end