{
  cambalache,
  fetchFromGitLab,
  python3,
  cmake,
  glib,
  gtk3,
  gtk4,
  gtksourceview5,
  webkitgtk_4_1,
  webkitgtk_6_0,
  libadwaita,
  libhandy,
  libxml2,
  linuxHeaders,
  casilda,
}:
(cambalache.overrideAttrs (prevAttrs: {
  src = fetchFromGitLab {
    domain = "gitlab.gnome.org";
    owner = "jpu";
    repo = "cambalache";
    rev = "1218e1c78a629c8529fc8107e3d7e8d3f0205176";
    hash = "sha256-qKu8UAzuXjJk1nC6SF1cF5SIGErQ+ocOtW7++qL/69U=";
  };

  pythonPath = with python3.pkgs; [
    pygobject3.dev
    lxml
  ];

  nativeBuildInputs = prevAttrs.nativeBuildInputs ++ [
    cmake
  ];

  buildInputs = [
    glib
    gtk3
    gtk4
    gtksourceview5
    webkitgtk_4_1
    webkitgtk_6_0
    # For extra widgets support.
    libadwaita
    libhandy
    libxml2
    python3.pkgs.pygobject3.dev
    python3.pkgs.lxml
    (casilda.overrideAttrs (prevAttrs: {
      version = "1.4.0";
      src = fetchFromGitLab {
        domain = "gitlab.gnome.org";
        owner = "jpu";
        repo = "casilda";
        tag = "1.4.0";
        hash = "sha256-bGhfi3AFEG0RxzpLiwxxlTRQy3boZqlWJXCvG0ZPCUA=";
      };

      env.NIX_CFLAGS_COMPILE = "-I${linuxHeaders}/include/drm";
    }))
  ];
}))
