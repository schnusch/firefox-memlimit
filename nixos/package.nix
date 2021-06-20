{ fetchFromGitHub
, lib
, rustPlatform
}:

rustPlatform.buildRustPackage rec {
  pname   = "firefox-memlimit";
  version = "0.1.0";

  src = fetchFromGitHub {
    owner = "schnusch";
    repo = pname;
    rev = "8869e9b43ce445db0989f0793613c3c121b65901";
    sha256 = "1baavqaxx9gnrvyq4hfng4cqi08f7bqcbw3f0v5f09gnsrcv4q56";
  };

  cargoSha256 = "1wx8a2syq9wad8ljadzw06xmcyv5fxiva2q9whpsz50kb5k42mqm";

  meta = with lib; {
    homepage = "https://github.com/schnusch/${pname}";
    license = licenses.gpl3;
  };
}
