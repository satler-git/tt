{
  description = "A very basic flake";

  # inputs = {
  #   nixpkgs.url = "github:nixos/nixpkgs?ref=nixpkgs-unstable";
  # };

  outputs = { self, nixpkgs }:
  let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
  in{
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = with pkgs; [
        mpv
        yt-dlp
        ffmpeg_7-full
      ];
    };
  };
}
