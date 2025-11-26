class Forge < Formula
  desc "Zero tokens. Zero emissions. YAML formula calculator with Excel bridge"
  homepage "https://github.com/royalbit/forge"
  url "https://github.com/royalbit/forge/archive/refs/tags/v3.1.0.tar.gz"
  sha256 "209eb61dee7e78907fd5958861d0bda53d1887fee35e69aac2f4a75fe700f317"
  license "MIT"
  head "https://github.com/royalbit/forge.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    # Create a simple test model
    (testpath/"test.yaml").write <<~EOS
      _forge_version: "1.0.0"
      inputs:
        price:
          value: 100
        quantity:
          value: 50
      outputs:
        revenue:
          formula: "=inputs.price * inputs.quantity"
          value: 5000
    EOS

    output = shell_output("#{bin}/forge validate #{testpath}/test.yaml")
    assert_match "valid", output.downcase
  end
end
