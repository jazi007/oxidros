{
  description = "oxidros: Native Rust ROS 2 client library";

  inputs = {
    nix-ros-overlay.url = "github:lopsided98/nix-ros-overlay";
    nixpkgs.follows = "nix-ros-overlay/nixpkgs";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nix-ros-overlay,
      nixpkgs,
      rust-overlay,
    }:
    nix-ros-overlay.inputs.flake-utils.lib.eachDefaultSystem (
      system:
      let
        # Supported ROS 2 distros via nix-ros-overlay.
        # Note: lyrical (May 2026) is brand-new — update nix-ros-overlay if it fails.
        distros = [
          "jazzy" # (May 2024 – May 2029, LTS) — default
          "humble" # (May 2022 – May 2027, LTS)
          "kilted" # (May 2025 – Nov 2026)
          "lyrical" # (May 2026 – Nov 2027)
        ];

        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            nix-ros-overlay.overlays.default
            rust-overlay.overlays.default
          ];
        };

        # Always latest stable Rust — no nightly needed.
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "llvm-tools-preview"
          ];
        };

        # ---------------------------------------------------------------------------
        # ROS package sets
        # ---------------------------------------------------------------------------

        # Minimal: exactly the packages oxidros-rcl's build.rs needs for
        # bindgen (headers) and link (libraries).  Sufficient to compile the
        # whole workspace with `--features rcl`.
        minimalPkgs =
          distro:
          with pkgs.rosPackages.${distro};
          [
            rcl
            rcl-interfaces
            rcl-action
            rcl-yaml-param-parser
            rcutils
            rmw
            rmw-implementation
            action-msgs
            builtin-interfaces
            rosidl-runtime-c
            rosidl-typesupport-interface
            rosidl-dynamic-typesupport # introduced in Jazzy; humble may omit
            service-msgs
            type-description-interfaces
            unique-identifier-msgs
          ];

        # Full: minimal + message packages, type-support generators, CLI tools,
        # and the Zenoh RMW — enough to run tests and use `ros2` tooling.
        fullPkgs =
          distro:
          with pkgs.rosPackages.${distro};
          minimalPkgs distro
          ++ [
            std-msgs
            geometry-msgs
            sensor-msgs
            example-interfaces
            common-interfaces
            composition-interfaces
            statistics-msgs
            lifecycle-msgs
            rosgraph-msgs
            rosidl-default-generators
            rosidl-default-runtime
            test-msgs
            rclpy
            ros2cli
            ament-cmake-core
            ament-cmake
            rmw-zenoh-cpp
            rmw-cyclonedds-cpp
          ];

        # Build a merged, unwrapped ROS environment store path.
        # wrapPrograms = false keeps Nix Store paths from being prepended to
        # PATH/LD_LIBRARY_PATH — the shellHook appends them instead (suffix logic).
        mkRosEnv =
          distro: variant:
          pkgs.rosPackages.${distro}.buildEnv {
            paths = if variant == "minimal" then minimalPkgs distro else fullPkgs distro;
            wrapPrograms = false;
          };

        # ---------------------------------------------------------------------------
        # Common tooling (non-ROS)
        # ---------------------------------------------------------------------------

        commonBuildInputs = with pkgs; [
          rustToolchain
          sccache
          clang
          llvmPackages.libclang
          llvmPackages.bintools
          pkg-config
          just
          python3
        ];

        # These are set both as mkShell attributes (exported by `nix print-dev-env`)
        # and carry over into build.rs through cargo's environment.
        commonEnvVars = {
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          CLANG_PATH = "${pkgs.llvmPackages.clang}/bin/clang";
          RUST_BACKTRACE = "1";
          RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";
        };

        # ---------------------------------------------------------------------------
        # Shell factory
        # ---------------------------------------------------------------------------

        mkDevShell =
          distro: variant:
          let
            rosEnv = mkRosEnv distro variant;
            pythonVer = pkgs.python3;
          in
          pkgs.mkShell (
            {
              name = "oxidros-${distro}-${variant}";
              packages = commonBuildInputs ++ [ rosEnv ];

              # KEY: append ROS store path to the END of each var so that a
              # sourced workspace `setup.bash` (prepended by ament) takes
              # precedence over the Nix-provided paths.
              # The ''${VAR:+$VAR:} pattern avoids a leading colon when the var
              # is empty (e.g. in a fresh shell before any setup.bash is sourced).
              shellHook = ''
                export LD_LIBRARY_PATH="''${LD_LIBRARY_PATH:+$LD_LIBRARY_PATH:}${rosEnv}/lib"
                export PYTHONPATH="''${PYTHONPATH:+$PYTHONPATH:}${rosEnv}/lib/${pythonVer.libPrefix}/site-packages"
                export CMAKE_PREFIX_PATH="''${CMAKE_PREFIX_PATH:+$CMAKE_PREFIX_PATH:}${rosEnv}"
                export AMENT_PREFIX_PATH="''${AMENT_PREFIX_PATH:+$AMENT_PREFIX_PATH:}${rosEnv}"
                export ROS_PACKAGE_PATH="''${ROS_PACKAGE_PATH:+$ROS_PACKAGE_PATH:}${rosEnv}/share"

                export ROS_DISTRO="${distro}"
                export ROS_VERSION=2
                export ROS_PYTHON_VERSION=3

                echo "oxidros | ROS 2 ${distro} (${variant}) | rust $(rustc --version | cut -d' ' -f2)"
              '';

              hardeningDisable = [ "all" ];
            }
            // commonEnvVars
          );

        # ---------------------------------------------------------------------------
        # Generate all 8 shells: ros-<distro>-<minimal|full>
        # ---------------------------------------------------------------------------

        allShells = builtins.listToAttrs (
          builtins.concatMap (
            distro:
            builtins.map (variant: {
              name = "ros-${distro}-${variant}";
              value = mkDevShell distro variant;
            }) [ "minimal" "full" ]
          ) distros
        );

      in
      {
        devShells = {
          default = allShells."ros-jazzy-full";
        } // allShells;

        formatter = pkgs.nixfmt-rfc-style;
      }
    );

  nixConfig = {
    extra-substituters = [ "https://ros.cachix.org" ];
    extra-trusted-public-keys = [
      "ros.cachix.org-1:dSyZxI8geDCJrwgvCOHDoAfOm5sV1wCPjBkKL+38Rvo="
    ];
  };
}
