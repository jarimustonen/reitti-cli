# Digitransit API access

`reitti` uses Digitransit production APIs. Those APIs require a free subscription key.

## Obtain a key

1. Open the [Digitransit API portal](https://portal-api.digitransit.fi/).
2. Select **Sign up** to register, or **Sign in** if you already have an account. Complete the requested account verification and two-factor authentication.
3. Open **Products**, then select **Digitransit developer API**.
4. Review and accept the product terms, complete the short subscription questionnaire, and select **Subscribe**. One subscription to this product grants access to its contained APIs; you do not need a separate subscription for each listed API.
5. Go to **Home → Profile**.
6. Find the subscription and select **Show** beside a key, then copy it into your secret store or shell environment.

Portal labels can change. Digitransit's [API portal and registration guide](https://digitransit.fi/en/developers/api-registration/) is the authoritative fallback.

## Configure `reitti`

Store the key without putting it in command arguments or shell history. If you already use a secret manager, have it write one line to `reitti config update --subscription-key-stdin`. No particular secret manager is required.

Without one, use this Bash-compatible hidden-input wrapper on macOS or Linux:

```bash
read -rsp 'Digitransit subscription key: ' key; printf '\n'
printf '%s\n' "$key" | reitti config update --subscription-key-stdin
unset key
```

The `reitti` process itself stays non-interactive: the caller supplies exactly one line on stdin. Confirm the saved value is redacted and run local diagnostics:

```sh
reitti --json config show
reitti doctor
```

Use `reitti doctor --online` only when you intend to make bounded API probes.

For short-lived automation, provide `DIGITRANSIT_SUBSCRIPTION_KEY` through the automation system's protected environment rather than an `export` command copied into shell history. Do not put a real key in a repository `.env` example, command-line argument, URL, log, issue, chat message, screenshot, or test fixture. Never commit or paste a key. `reitti` sends the value in the `digitransit-subscription-key` HTTP header, not in the URL.

## Primary and secondary keys

Each subscription has a primary and a secondary key with equivalent access. The pair allows rotation without downtime; it is not a requirement to send both keys or alternate between them. Configure one active key at a time.

To rotate safely:

1. Keep clients using the current primary key.
2. In **Home → Profile**, reveal and deploy the secondary key to every client.
3. Confirm requests work with the secondary key.
4. Select **Regenerate** for the old primary key. Regeneration invalidates its previous value immediately.
5. Optionally make the new primary key active and then regenerate the secondary key using the same staged process.

If a key may have been exposed, switch clients to the other key and regenerate the exposed key promptly. Do not regenerate both keys at once unless an intentional outage is acceptable.
