# AuthIt!

This is a simple user-management client to be used with Kanidm. I created it
very quickly for my own use, so that I wouldn't have to rely on the Kanidm CLI
so much, and it's probably not useful for you.

It started as a big layer of slop that I have been artisanally reviewing and
refactoring, and I provide no guarantees of correctness or security.

## Configuration

Configuration can be done with either environment variables or a toml file, or
a mix. A configuration option of `foo_bar` would be set by the environment
variable `AUTHIT_FOO_BAR`.


| Config key | Description |
| --- | ---|
| kanidm_url | The URL for your Kanidm server |
| kanidm_token | The service account API token. It will need read-write privileges to make changes. |
| oauth_client_id | The Kanidm oauth2 client id for AuthIt! |
| oauth_client_secret | The Kanidm oauth2 client secret for AuthIt! |
| oauth_redirect_uri | The AuthIt! redirect URI. This should be `https://{AUTHIT_DOMAIN}/auth/callbacl` |
| signing_secret | The secret AuthIt! uses to sign sessions and provision links. Run `openssl rand -hex 32` or similar to generate. | 
| admin_group | The group a user needs to be in to use this service. NOTE: Any user in this group will be able to create and delete users, and assign them to groups of their choice. | 
| data_dir | The directory to store a sqlite database or anything else AuthIt needs.|
| db_secret | The secret used to encrypt the sqlite database. Run `openssl rand -hex 32` or similar to generate. |
| log_level | Defaults to INFO. |
