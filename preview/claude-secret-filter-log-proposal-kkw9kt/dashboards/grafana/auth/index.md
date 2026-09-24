# Authentication




# Authentication

A fresh install has exactly one account: the admin, with a generated password.
That is fine behind `kubectl port-forward` and not fine the moment Grafana is [reachable](../architecture/#reaching-grafana) — every datasource behind it reads every metric in Thanos and every log in the tenant.

The chart does not model authentication itself.
It passes `grafana.ini` through verbatim, so **any** authentication Grafana supports is configurable from values, and Grafana's own documentation stays the reference.
This page covers the wiring that is specific to running Grafana from this chart; follow the links for the provider details.

## Where the configuration goes

Everything lives under `grafana.grafana.ini`, which deep-merges over the chart's own keys.
Section names carry dots, which is fine — they are map keys, not paths:

```yaml
grafana:
  grafana.ini:
    auth.generic_oauth:
      enabled: true
```

| Provider | `grafana.ini` section | Reference |
|---|---|---|
| Generic OIDC / OAuth2 (Okta, Auth0, Keycloak, Ory, Entra ID) | `auth.generic_oauth` | [Generic OAuth](https://grafana.com/docs/grafana/latest/setup-grafana/configure-security/configure-authentication/generic-oauth/) |
| GitHub, GitLab, Google, Azure AD, Okta | `auth.github`, `auth.gitlab`, `auth.google`, `auth.azuread`, `auth.okta` | [Configure authentication](https://grafana.com/docs/grafana/latest/setup-grafana/configure-security/configure-authentication/) |
| SAML | `auth.saml` | [SAML](https://grafana.com/docs/grafana/latest/setup-grafana/configure-security/configure-authentication/saml/) — Grafana Enterprise / Cloud only |
| LDAP | `auth.ldap` plus the subchart's `ldap` block | [LDAP](https://grafana.com/docs/grafana/latest/setup-grafana/configure-security/configure-authentication/ldap/) |
| Reverse proxy / mesh-terminated | `auth.proxy` | [Auth proxy](https://grafana.com/docs/grafana/latest/setup-grafana/configure-security/configure-authentication/auth-proxy/) |
| Anonymous | `auth.anonymous` | [Anonymous access](https://grafana.com/docs/grafana/latest/setup-grafana/configure-security/configure-authentication/grafana/#anonymous-authentication) — refused at render time on an exposed Grafana |

Prefer the generic OIDC section over a provider-specific one unless you need something only the latter has.
It works against every IdP that speaks OIDC, and it moves cleanly when the IdP changes.

## Client secrets never go in `grafana.ini`

`grafana.ini` renders into a **ConfigMap**.
A secret written there is plaintext in the release manifest, in `helm get values`, and in whatever Git repo holds your values file.

Grafana reads secrets from disk or the environment instead:

| | |
|---|---|
| `$__file{/path}` | reads a mounted Secret at startup — the better default, and the same mechanism the database password uses |
| `$__env{VAR}` | reads an environment variable injected with `grafana.envValueFrom` |

The subchart's `assertNoLeakedSecrets` check fails the render if a known-sensitive key is set to a literal, including every `client_secret`.
Leave it on.

The chart does not create the Secret — provision it with External Secrets Operator, Vault Agent, SOPS, or your cloud's CSI driver.
It must exist in the namespace the Grafana **pod** runs in, which under `split-namespace` is `grafana`, not the release namespace.

## A complete OIDC example

```yaml
grafana:
  grafana.ini:
    server:
      # Must match the URL users reach. The OAuth redirect URI is built from it,
      # and a mismatch fails the callback rather than the login.
      root_url: https://grafana.example.com
    auth:
      # Send users straight to the IdP instead of showing the login form.
      oauth_auto_login: true
      disable_login_form: true
    auth.generic_oauth:
      enabled: true
      name: SSO
      allow_sign_up: true
      client_id: <client-id>
      client_secret: $__file{/etc/secrets/grafana-oidc/client-secret}
      scopes: openid profile email groups
      auth_url: https://idp.example.com/oauth2/v1/authorize
      token_url: https://idp.example.com/oauth2/v1/token
      api_url: https://idp.example.com/oauth2/v1/userinfo
      allowed_domains: example.com
      # Group membership decides the role, so nobody is provisioned by hand.
      role_attribute_path: contains(groups[*], 'sre') && 'Admin' || 'Viewer'

  extraSecretMounts:
    - name: grafana-oidc
      secretName: grafana-oidc
      mountPath: /etc/secrets/grafana-oidc
      readOnly: true
```

Register `https://grafana.example.com/login/generic_oauth` as the redirect URI with the IdP.

## User provisioning is role mapping

Grafana creates a user on first login when `allow_sign_up` is true; there is no separate provisioning step to run.
What you do have to decide is what role that user gets, and the answer should be an IdP group rather than a manual grant.

`role_attribute_path` is a [JMESPath](https://jmespath.org/) expression over the claims returned by `api_url`, evaluating to `Viewer`, `Editor`, `Admin`, or `GrafanaAdmin`:

```
contains(groups[*], 'platform-admins') && 'Admin' || contains(groups[*], 'oncall') && 'Editor' || 'Viewer'
```

Two things that catch people out:

* **The claim has to be in the response.** `groups` is only present if the `groups` scope is requested *and* the IdP is configured to emit it. If the expression evaluates to nothing, Grafana falls back to `auto_assign_org_role` (`Viewer` by default), which looks like the mapping silently not working.
* **`GrafanaAdmin` needs `allow_assign_grafana_admin: true`** as well, or the expression can never grant it.

Set `role_attribute_strict: true` once the mapping is right — it refuses the login rather than falling back to the default role, which turns a misconfigured claim into a visible failure instead of a quiet privilege change.

For finer-grained control than the four built-in roles, see [role-based access control](https://grafana.com/docs/grafana/latest/administration/roles-and-permissions/access-control/) — Grafana Enterprise and Cloud only.

## Keep a break-glass path

`disable_login_form: true` hides the form; it does not remove it.
`https://grafana.example.com/login?disableAutoLogin` still reaches it, and the generated admin still works.

Know that before you need it — an IdP outage with no local login is an outage of your incident tooling during an incident.
The admin password is in a Secret the chart or the Terraform module owns:

```bash
kubectl --namespace monitoring get secret grafana -o jsonpath="{.data.admin-password}" | base64 -d
```

## What Grafana roles do not do

Grafana's roles govern the Grafana UI, not the data behind it.
Every datasource is queryable by anyone who can reach Grafana, so a Viewer still reads every metric in Thanos and every log in the tenant — the Explore view alone is enough.

The hard isolation boundary is the **install**, not the role.
See [Tenancy & auth](../../../operating/production-best-practices/#9-tenancy--auth) for what that means for the logging backend, and [Datasources](../architecture/#datasources) for what each one exposes.

## See also

* [Reaching Grafana](../architecture/#reaching-grafana) — expose it before you need any of this, and TLS before that.
* [Production Best Practices > Grafana](../../../operating/production-best-practices/#3-authentication-and-authorization) — the checklist form.
* [Configure authentication](https://grafana.com/docs/grafana/latest/setup-grafana/configure-security/configure-authentication/) (official) — every provider, in depth.
* [Configure Grafana](https://grafana.com/docs/grafana/latest/setup-grafana/configure-grafana/) (official) — every `grafana.ini` key.

