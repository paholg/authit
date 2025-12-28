{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.authit;
in
{
  options.services.authit = {
    enable = lib.mkEnableOption "authit user management service";

    package = lib.mkPackageOption pkgs "authit" { };

    # Non-secret configuration
    kanidmUrl = lib.mkOption {
      type = lib.types.str;
      description = "URL of the Kanidm server";
      example = "https://auth.example.com";
    };

    oauthClientId = lib.mkOption {
      type = lib.types.str;
      description = "OAuth2 client ID";
    };

    authitUrl = lib.mkOption {
      type = lib.types.str;
      description = "Public URL of this AuthIt instance";
      example = "https://authit.example.com";
    };

    logLevel = lib.mkOption {
      type = lib.types.enum [
        "trace"
        "debug"
        "info"
        "warn"
        "error"
      ];
      default = "info";
      description = "Log level for the service";
    };

    adminGroup = lib.mkOption {
      type = lib.types.str;
      default = "authit_admin";
      description = "Kanidm group name for admin access";
    };

    ipAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "Bind address";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
      description = "Port to listen on";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Open firewall port";
    };

    configFile = lib.mkOption {
      type = lib.types.path;
      description = ''
        Path to the AuthIt configuration file containing secrets or other config.
      '';
      example = "/run/secrets/authit.toml";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.authit = {
      description = "AuthIt user management service";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      environment = {
        AUTHIT_CONFIG_PATH = cfg.configFile;
        AUTHIT_KANIDM_URL = cfg.kanidmUrl;
        AUTHIT_OAUTH_CLIENT_ID = cfg.oauthClientId;
        AUTHIT_AUTHIT_URL = cfg.authitUrl;
        AUTHIT_ADMIN_GROUP = cfg.adminGroup;
        AUTHIT_LOG_LEVEL = cfg.logLevel;
        AUTHIT_DATA_DIR = "/var/lib/authit";
        IP = cfg.ipAddress;
        PORT = toString cfg.port;
      };

      serviceConfig = {
        ExecStart = "${cfg.package}/bin/web";
        DynamicUser = true;
        StateDirectory = "authit";

        # Hardening
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictNamespaces = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
      };
    };

    networking.firewall.allowedTCPPorts = lib.mkIf cfg.openFirewall [ cfg.port ];
  };
}
