# AI PR reviewer — open-source options (whole GitHub account, auto-review on open)

## TL;DR

- **Path 1 — per-repo Action, one workflow file per repo (recommended):**
  [`The-PR-Agent/pr-agent`](https://github.com/qodo-ai/pr-agent) as a
  GitHub Action with `on: pull_request: types: [opened, reopened,
  ready_for_review, synchronize]` plus `issue_comment` for `/review`-style
  commands. It is MIT-licensed
  ([LICENSE](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/LICENSE)),
  supports the widest provider list via LiteLLM (OpenAI, Anthropic, Gemini,
  custom OpenAI-compatible `api_base`, Ollama/VLLM — all verified in
  [changing_a_model.md](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/docs/docs/usage-guide/changing_a_model.md)),
  and was released as
  [v0.45.0 on 2026-09-05](https://github.com/The-PR-Agent/pr-agent/releases/tag/v0.45.0)
  (~12.9k stars). Scale it with a central reusable workflow (`on:
  workflow_call:`) so each repo only carries a thin caller file —
  reusable workflows work on all plans including Free
  ([docs](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows)).
- **Path 2 — one self-hosted GitHub App for the whole account:**
  [`anc95/ChatGPT-CodeReview`](https://github.com/anc95/ChatGPT-CodeReview)
  in server mode (Probot-based, subscribes to
  [`pull_request.opened` /
  `pull_request.synchronize`](https://raw.githubusercontent.com/anc95/ChatGPT-CodeReview/main/src/bot.ts)),
  ISC-licensed
  ([LICENSE](https://raw.githubusercontent.com/anc95/ChatGPT-CodeReview/main/LICENSE)),
  active (last push 2026-08-10). A personal account can register an app
  under itself and install it on all repos
  ([docs](https://docs.github.com/en/apps/creating-github-apps/registering-a-github-app/registering-a-github-app),
  [docs](https://docs.github.com/en/apps/using-github-apps/installing-your-own-github-app)).
  Provider breadth is narrower than pr-agent (OpenAI + custom endpoint +
  Azure + GitHub Models per
  [src/chat.ts](https://raw.githubusercontent.com/anc95/ChatGPT-CodeReview/main/src/chat.ts)).
- **Path 3 — DIY webhook reviewer (only if neither fits):**
  [`probot/probot`](https://github.com/probot/probot) (ISC) plus a LiteLLM
  gateway ([`BerriAI/litellm`](https://github.com/BerriAI/litellm), MIT
  outside `enterprise/`) as the multi-provider layer. Full control, but you
  own hosting, secrets, and prompt engineering.
- **Recommendation:** pr-agent as a GitHub Action, auto-review on PR open,
  with a central reusable workflow + thin per-repo callers, LLM routed
  through LiteLLM (built into pr-agent) or a self-hosted LiteLLM proxy.
  It is the only option that is simultaneously open-source, multi-provider,
  auto-on-open, and actively maintained. Runner-up: cr-gpt server mode if
  you would rather run one always-on app than N workflow files.
- **Method note:** `web_search` was used only to discover candidates; every
  claim below was verified by reading the primary source (repo README,
  LICENSE, `action.yml`/workflow YAML, source, or docs page) and links
  to the file it came from.

## Trigger topology: how "every PR on the account" actually fires

### (a) GitHub Actions per repo

- `on: pull_request` with no `types:` fires only on `opened`,
  `synchronize`, `reopened`; every other activity type needs an explicit
  `types:` keyword (same default for `pull_request_target`) —
  [events reference](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request).
  Real reviewers use exactly this shape, e.g. `types: [opened, synchronize,
  reopened]` —
  [events reference](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request),
  [ai-pr-review README](https://github.com/tag1consulting/ai-pr-review).
- Baseline cost: each repo needs its own `.github/workflows/*.yml`
  (reusable-workflow files live in `.github/workflows/`; subdirectories
  unsupported), so N personal repos = N files to plant and update —
  [reusable-workflows docs](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows).
  The cheap scaling trick is a **reusable workflow**: define once with `on:
  workflow_call:` (+ inputs/secrets), call via
  `jobs.<id>.uses: {owner}/{repo}/.github/workflows/{file}@{ref}`, pass
  with `with:`/`secrets:` — available on all plans including Free, pin
  `@sha` for security —
  [docs](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows).
- Org-level enforcement is **not** available to a personal account: org
  rulesets require GitHub Team (multi-repo) / Enterprise (org-level) —
  [about rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets)
  — and the old Actions "Required Workflows" beta was migrated into
  Repository Rules with pass-gating "only available on GitHub Enterprise
  plans" —
  [changelog](https://github.blog/changelog/2023-08-02-github-actions-required-workflows-will-move-to-repository-rules/).
  Net: no Team/Enterprise plan and no org means **per-repo file (full or
  thin caller) is the only mechanism**.
- `pull_request` vs `pull_request_target` (fork safety): `pull_request`
  runs the workflow from the merge commit with a read-only `GITHUB_TOKEN`
  and no other secrets on forks; `pull_request_target` runs in the base
  repo's default-branch context with a read/write token **and** secrets —
  so checking out + executing fork head under it is a "pwn request"
  (cache-poisoning, secret exfiltration). Prefer `pull_request`; never
  execute checked-out fork code; restrict secrets —
  [secure use of `pull_request_target`](https://docs.github.com/en/actions/reference/security/securely-using-pull_request_target),
  [`pull_request_target` reference](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request_target).

### (b) Self-hosted GitHub App / webhook server, installed account-wide

- "You can register a GitHub App under your personal account or under any
  organization you own" (no org required; 100 apps per user, unlimited
  installs) —
  [registering an app](https://docs.github.com/en/apps/creating-github-apps/registering-a-github-app/registering-a-github-app).
  Install your own app via profile → Settings → Developer settings →
  GitHub Apps → Install App → "Install on your account", choosing "All
  repositories" or "Only select repositories" —
  [installing your own app](https://docs.github.com/en/apps/using-github-apps/installing-your-own-github-app).
  "You can install a GitHub App on your personal repository" —
  [GitHub Apps vs OAuth](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/differences-between-github-apps-and-oauth-apps).
  One install with "All repositories" = one trigger surface for the whole
  account, no per-repo files.
- Security basics for the app (all from GitHub docs): set a webhook
  secret and verify `X-Hub-Signature-256` (HMAC-SHA256 hex, `sha256=`
  prefix) with a constant-time compare, never `==` —
  [validating deliveries](https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries).
  App auth = RSA private key → short-lived JWT (`RS256`, `exp` ≤ 10 min)
  → 1-hour installation token for API calls —
  [JWT auth](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-json-web-token-jwt-for-a-github-app),
  [auth overview](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/about-authentication-with-a-github-app).
  Least privilege: apps start with no permissions, pick the minimum; a
  reviewer bot needs Pull requests (read diff, read+write to post
  reviews), Contents read, Checks read+write only if emitting check runs —
  [choosing permissions](https://docs.github.com/en/apps/creating-github-apps/registering-a-github-app/choosing-permissions-for-a-github-app).
  Prefer a GitHub App over OAuth: fine-grained perms, short-lived install
  tokens, one central webhook for all installed repos, rate limits that
  scale, actions attributed to `@app[bot]` —
  [GitHub Apps vs OAuth](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/differences-between-github-apps-and-oauth-apps),
  [when to build an app](https://docs.github.com/en/apps/creating-github-apps/about-creating-github-apps/deciding-when-to-build-a-github-app).

### (c) Where the credentials live: Actions has no profile-level secret store

- Actions secrets are scoped to repository, environment, or organization
  only; the sole user-level secret type is Codespaces-scoped ("user:
  available to Codespaces for your user" —
  [gh secret manual](https://cli.github.com/manual/gh_secret_set)). A
  personal account therefore has no profile-wide Actions credential, and
  a repo inherits secrets only from its **parent organization**. On
  GitHub Free, organization-level secrets are **not accessible by
  private repositories** (public repos only; a paid plan unlocks
  private) —
  [secrets doc](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/use-secrets).
  Pragmatic Actions pattern without an org: one scripted pass across
  repos — `gh secret set OPENAI_KEY --repo OWNER/REPO` per repo, value
  from stdin or `--body`
  ([same doc](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/use-secrets),
  [gh manual](https://cli.github.com/manual/gh_secret_set)). Note secrets
  are not automatically passed into reusable workflows — thin callers
  must pass them explicitly
  ([same doc](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/use-secrets)).
  `GITHUB_TOKEN` needs none of this: it is minted per workflow run.
- The self-hosted GitHub App path is the true "whole profile" option:
  the LLM key(s) plus `APP_ID` / `PRIVATE_KEY` / `WEBHOOK_SECRET` live
  only in the server's env — never in any repo. cr-gpt server mode is
  provisioned exactly this way (clone → `.env` → run,
  [README](https://github.com/anc95/ChatGPT-CodeReview)); pr-agent
  offers the same self-hosted App/webhook deployment
  ([README](https://github.com/qodo-ai/pr-agent)).

## Candidate table

| Tool | License | Deploy mode | Auto-trigger on open | Providers | Activity (checked 2026-09-06) |
|---|---|---|---|---|---|
| `The-PR-Agent/pr-agent` | [MIT](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/LICENSE) | [Action](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/action.yaml) + [App](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/docs/docs/installation/github.md) + [CLI/Docker/webhooks](https://github.com/qodo-ai/pr-agent) | Yes: [`auto_review`/`pr_commands`](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/pr_agent/settings/configuration.toml) + comment commands | Broadest: OpenAI + custom `api_base`, Azure, Anthropic, Gemini/Vertex, Bedrock, Ollama/VLLM, OpenRouter, +more via built-in LiteLLM ([doc](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/docs/docs/usage-guide/changing_a_model.md)) | [v0.45.0, 2026-09-05](https://github.com/The-PR-Agent/pr-agent/releases/tag/v0.45.0); ~12.9k★ |
| `anthropics/claude-code-action` | [MIT](https://raw.githubusercontent.com/anthropics/claude-code-action/main/LICENSE) | [Action only](https://raw.githubusercontent.com/anthropics/claude-code-action/main/action.yml) (composite/Bun, runs on your runner) | Yes, as an explicit workflow pattern: [`claude-review.yml`](https://raw.githubusercontent.com/anthropics/claude-code-action/main/.github/workflows/claude-review.yml) (`types: [opened]` + `/review-pr` prompt); default is `@claude` mention | Claude-only; 4 auth paths: direct API, Bedrock, Vertex, Foundry ([doc](https://raw.githubusercontent.com/anthropics/claude-code-action/main/docs/cloud-providers.md)) | [v1.0.217, 2026-09-06](https://github.com/anthropics/claude-code-action/releases/tag/v1.0.217); ~8.8k★ |
| `anc95/ChatGPT-CodeReview` | [ISC](https://raw.githubusercontent.com/anc95/ChatGPT-CodeReview/main/LICENSE) | [Action + self-hosted server](https://github.com/anc95/ChatGPT-CodeReview) (Probot-based, `src/bot.ts`) | Yes: Action [`types: [opened, reopened, synchronize]`](https://raw.githubusercontent.com/anc95/ChatGPT-CodeReview/main/.github/workflows/cr.yml); server on [`pull_request.opened/synchronize`](https://raw.githubusercontent.com/anc95/ChatGPT-CodeReview/main/src/bot.ts) | OpenAI + custom `OPENAI_API_ENDPOINT`, Azure, GitHub Models ([src/chat.ts](https://raw.githubusercontent.com/anc95/ChatGPT-CodeReview/main/src/chat.ts)) | Last push 2026-08-10; 4,462★ ([API](https://api.github.com/repos/anc95/ChatGPT-CodeReview)) |
| `tag1consulting/ai-pr-review` | [MIT](https://raw.githubusercontent.com/tag1consulting/ai-pr-review/main/LICENSE) | [Action only](https://raw.githubusercontent.com/tag1consulting/ai-pr-review/main/action.yml) (container on GHCR) | Yes: `types: [opened, synchronize, reopened]` per [README](https://github.com/tag1consulting/ai-pr-review) | `anthropic \| openai \| openai-compatible \| google \| bedrock-proxy` ([action.yml](https://raw.githubusercontent.com/tag1consulting/ai-pr-review/main/action.yml)) | Active 2026 (repo header reports MIT; verify releases page before pinning) |
| `keithah/multi-provider-code-review` | [MIT](https://github.com/keithah/multi-provider-code-review) | Action ([node20](https://raw.githubusercontent.com/keithah/multi-provider-code-review/main/action.yml)) + self-hosted Docker/webhook (per README) | Yes: `on: pull_request:` per [README](https://github.com/keithah/multi-provider-code-review) | `openrouter/*` incl. `openrouter/free`, OAuth-CLI `claude/` `codex/` `gemini/` `opencode/` (per README) | Tiny/newest: 4★/1 fork at check — least-mature pick |
| `reviewdog/reviewdog` | [MIT](https://raw.githubusercontent.com/reviewdog/reviewdog/master/LICENSE) | Plumbing inside Actions (via [action-setup](https://github.com/reviewdog/action-setup)) — **not** an LLM reviewer | N/A (posts whatever tool output it is piped) | N/A | [v0.21.0, 2025-09-03](https://raw.githubusercontent.com/reviewdog/reviewdog/master/CHANGELOG.md); ~9.6k★ |
| `BerriAI/litellm` | [MIT outside `enterprise/`](https://raw.githubusercontent.com/BerriAI/litellm/main/LICENSE) | Python SDK / proxy server — provider layer, not a reviewer | N/A | 100+ via OpenAI-compatible API ([README](https://github.com/BerriAI/litellm), [providers](https://docs.litellm.ai/docs/providers)) | ~58k★, adopters incl. Stripe/Netflix ([README](https://github.com/BerriAI/litellm)) |
| `probot/probot` | [ISC](https://raw.githubusercontent.com/probot/probot/master/LICENSE) | DIY GitHub App framework (Node/TS; `app.on(event, cb)`, serverless or Actions adapter — [deployment](https://raw.githubusercontent.com/probot/probot/master/docs/deployment.md)) | You build it (`pull_request` events via [`app.on`](https://raw.githubusercontent.com/probot/probot/master/docs/webhooks.md)) | Whatever LLM you wire in | ~9.6k★ |
| `All-Hands-AI/OpenHands` | [MIT](https://raw.githubusercontent.com/All-Hands-AI/OpenHands/main/LICENSE) | Self-hosted agent stack (Agent Canvas automation, not an Action) | Via the documented [PR Review Assistant](https://docs.openhands.dev/openhands/usage/agent-canvas/prebuilt/github-pr-review) automation (conversational setup, no YAML) | Any LLM via profiles ([README](https://github.com/All-Hands-AI/OpenHands)) | ~86k★ but mid-pivot to Agent Canvas ([README](https://github.com/All-Hands-AI/OpenHands)) |

## Details per tool

### 1. `The-PR-Agent/pr-agent` — the default pick

Community-maintained fork; `qodo-ai/pr-agent` redirects to
`The-PR-Agent/pr-agent` (release pages live under `The-PR-Agent`, e.g.
[v0.45.0](https://github.com/The-PR-Agent/pr-agent/releases/tag/v0.45.0));
the README calls it a "community-maintained legacy project of Qodo"
([README](https://github.com/qodo-ai/pr-agent)). MIT —
[LICENSE](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/LICENSE).

- **Deploy:** GitHub Action (Docker action, "Recommended") per
  [action.yaml](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/action.yaml)
  and [README](https://github.com/qodo-ai/pr-agent); GitHub App install
  per [installation/github.md](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/docs/docs/installation/github.md);
  CLI (`pip install pr-agent`, `pr-agent --pr_url ... review`), Docker,
  self-hosted, webhooks; also GitLab/Bitbucket/Azure/Gitea
  ([README](https://github.com/qodo-ai/pr-agent),
  [usage guide](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/docs/docs/usage-guide/automations_and_usage.md)).
- **Triggers:** Action-side `github_action_config.auto_review /
  auto_describe / auto_improve` + `pr_actions` event list; App-side
  `[github_app] pr_commands = ["/describe", "/review", "/improve"]` on
  open/reopen/ready-for-review; kill-switch `[config]
  disable_auto_feedback = true` — all in
  [configuration.toml](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/pr_agent/settings/configuration.toml)
  and the
  [usage guide](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/docs/docs/usage-guide/automations_and_usage.md).
  (Note: the repo-root `.pr_agent.toml` is *not* the OSS config — its own
  header says it is consumed by Qodo's hosted agent, not the open-source
  one:
  [.pr_agent.toml](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/.pr_agent.toml).)
- **Providers:** LiteLLM-routed (LiteLLM v1.99.0 per
  [v0.45.0 notes](https://github.com/The-PR-Agent/pr-agent/releases/tag/v0.45.0));
  OpenAI + custom `api_base`, Azure, Anthropic, Gemini (AI Studio),
  Vertex, Bedrock, Ollama/VLLM, OpenRouter, HuggingFace, Replicate, Groq,
  Mistral, DeepSeek, xAI, and more — full list in
  [changing_a_model.md](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/docs/docs/usage-guide/changing_a_model.md).
  Default model `gpt-5.6` per
  [configuration.toml](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/pr_agent/settings/configuration.toml).
- **Activity:** [v0.45.0 released 2026-09-05](https://github.com/The-PR-Agent/pr-agent/releases/tag/v0.45.0)
  (README "News" still lists v0.41.0 — stale, prefer the releases page);
  ~12.9k stars / ~1.8k forks at check.
- **Minimal setup** (from
  [installation/github.md](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/docs/docs/installation/github.md)):

```yaml
# .github/workflows/pr_agent.yml
on:
  pull_request:
    types: [opened, reopened, ready_for_review, synchronize]
  issue_comment:
jobs:
  pr_agent_job:
    if: ${{ github.event.sender.type != 'Bot' }}
    runs-on: ubuntu-latest
    permissions:
      issues: write
      pull-requests: write
      contents: write
      checks: write
    steps:
      - name: PR Agent action step
        id: pragent
        uses: the-pr-agent/pr-agent@main
        env:
          OPENAI_KEY: ${{ secrets.OPENAI_KEY }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

Point it at any provider by setting the model/`api_base` per
[changing_a_model.md](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/docs/docs/usage-guide/changing_a_model.md).

### 2. `anthropics/claude-code-action` — best if you standardize on Claude

MIT ([LICENSE](https://raw.githubusercontent.com/anthropics/claude-code-action/main/LICENSE));
"executes entirely on your own GitHub runner"
([README](https://github.com/anthropics/claude-code-action)).

- **Deploy:** GitHub Action only — composite/Bun action per
  [action.yml](https://raw.githubusercontent.com/anthropics/claude-code-action/main/action.yml).
  The `/install-github-app` quickstart only provisions auth secrets, not a
  separate app distribution.
- **Triggers:** `@claude` mention by default (`trigger_phrase`, plus
  `assignee_trigger` / `label_trigger`) per
  [action.yml](https://raw.githubusercontent.com/anthropics/claude-code-action/main/action.yml);
  automatic review on open is an explicit workflow pattern — the repo's own
  [claude-review.yml](https://raw.githubusercontent.com/anthropics/claude-code-action/main/.github/workflows/claude-review.yml)
  fires on `pull_request: types: [opened]` (skipping forks) with a
  `/review-pr` prompt; "Automatic PR Code Review" is also the README's
  first Solutions Guide entry. Reviews run subagent specialists
  (code-quality, performance, test-coverage, docs, security) via
  [review-pr.md](https://raw.githubusercontent.com/anthropics/claude-code-action/main/.claude/commands/review-pr.md).
- **Providers:** Claude-only; four auth paths — direct Anthropic API
  (key/OAuth/federation), Bedrock (`use_bedrock`), Vertex (`use_vertex`),
  Foundry (`use_foundry`) — per
  [action.yml](https://raw.githubusercontent.com/anthropics/claude-code-action/main/action.yml)
  and [cloud-providers.md](https://raw.githubusercontent.com/anthropics/claude-code-action/main/docs/cloud-providers.md).
  No OpenAI/Gemini/local option.
- **Activity:** [v1.0.217 released 2026-09-06](https://github.com/anthropics/claude-code-action/releases/tag/v1.0.217)
  (rapid patch cadence); ~8.8k stars / ~2.1k forks at check.
- **Minimal setup — auto review** (from
  [claude-review.yml](https://raw.githubusercontent.com/anthropics/claude-code-action/main/.github/workflows/claude-review.yml)):

```yaml
name: PR Review
on:
  pull_request:
    types: [opened]
jobs:
  review:
    if: github.event.pull_request.head.repo.full_name == github.repository
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write
      id-token: write
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 1
      - uses: anthropics/claude-code-action@v1
        with:
          anthropic_api_key: ${{ secrets.ANTHROPIC_API_KEY }}
          prompt: "/review-pr REPO: ${{ github.repository }} PR_NUMBER: ${{ github.event.pull_request.number }}"
```

### 3. `anc95/ChatGPT-CodeReview` (cr-gpt) — one app for the whole account

ISC ([LICENSE](https://raw.githubusercontent.com/anc95/ChatGPT-CodeReview/main/LICENSE)).

- **Deploy:** GitHub Action (`action.yml` runs `action/index.cjs` on
  `node24`) **and** self-hosted server (clone → `.env` → `pm2 start`,
  plus `Dockerfile`; Probot-based — `app.yml`,
  [src/bot.ts](https://raw.githubusercontent.com/anc95/ChatGPT-CodeReview/main/src/bot.ts))
  per [README](https://github.com/anc95/ChatGPT-CodeReview). A hosted test
  bot ([apps/cr-gpt](https://github.com/apps/cr-gpt)) exists but is
  rate-limited, so self-deploy.
- **Triggers:** auto on open and on push: "automatically do the code
  review when you create a new Pull request… After `git push`… re-review"
  ([README](https://github.com/anc95/ChatGPT-CodeReview)); server on
  `pull_request.opened` + `pull_request.synchronize`
  ([src/bot.ts](https://raw.githubusercontent.com/anc95/ChatGPT-CodeReview/main/src/bot.ts));
  Action on `types: [opened, reopened, synchronize]`
  ([cr.yml](https://raw.githubusercontent.com/anc95/ChatGPT-CodeReview/main/.github/workflows/cr.yml)).
  No comment trigger; `TARGET_LABEL` gating hinted in
  [.env.example](https://raw.githubusercontent.com/anc95/ChatGPT-CodeReview/main/.env.example).
- **Providers:** OpenAI with custom `OPENAI_API_ENDPOINT` (any
  OpenAI-compatible base URL), Azure OpenAI, and GitHub Models
  (`USE_GITHUB_MODELS=true`, default `openai/gpt-4o-mini`) per
  [src/chat.ts](https://raw.githubusercontent.com/anc95/ChatGPT-CodeReview/main/src/chat.ts);
  tunable `MODEL/LANGUAGE/PROMPT/temperature/top_p/max_tokens/…`.
- **Activity:** 4,462 stars / 459 forks, last push 2026-08-10, updated
  2026-09-05 ([API](https://api.github.com/repos/anc95/ChatGPT-CodeReview));
  dogfoods itself via
  [cr.yml](https://raw.githubusercontent.com/anc95/ChatGPT-CodeReview/main/.github/workflows/cr.yml).
- **Minimal setup** (from
  [README](https://github.com/anc95/ChatGPT-CodeReview)):

```yaml
on:
  pull_request:
    types: [opened, reopened, synchronize]
jobs:
  review:
    runs-on: ubuntu-latest
    steps:
      - uses: anc95/ChatGPT-CodeReview@main
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
```

### 4. `tag1consulting/ai-pr-review` — multi-agent MIT alternative

MIT ([LICENSE](https://raw.githubusercontent.com/tag1consulting/ai-pr-review/main/LICENSE));
"AI-powered pull request review using multiple LLM agents… summary
comment and inline findings" ([README](https://github.com/tag1consulting/ai-pr-review)).

- **Deploy:** GitHub Action only (container on GHCR, amd64/arm64).
- **Triggers:** `on: pull_request: types: [opened, synchronize,
  reopened]` quickstart with `contents: read, pull-requests: write`
  ([README](https://github.com/tag1consulting/ai-pr-review)). No
  App/webhook mode — pure Action.
- **Providers:** `provider` input = `anthropic | openai |
  openai-compatible | google | bedrock-proxy` with per-provider secret +
  default models, per [README](https://github.com/tag1consulting/ai-pr-review)
  and [action.yml](https://raw.githubusercontent.com/tag1consulting/ai-pr-review/main/action.yml).
  Distinctive: quick mode (summarizer, code-reviewer, silent-failure
  hunter) + full mode (+architecture, security, blind-hunter, edge-case,
  adversarial), deterministic analyzers (shellcheck, semgrep, trufflehog,
  …), CVE via OSV.dev, severity → REQUEST_CHANGES/APPROVE mapping.

### 5. `keithah/multi-provider-code-review` — consensus multi-provider, immature

MIT ([repo](https://github.com/keithah/multi-provider-code-review));
"Hybrid AST + LLM GitHub Action that fuses multiple AI providers with
consensus filtering, cost tracking, and security scanning" ([README](https://github.com/keithah/multi-provider-code-review)).

- **Deploy:** Action (`node20`, `dist/index.js` per
  [action.yml](https://raw.githubusercontent.com/keithah/multi-provider-code-review/main/action.yml))
  + self-hosted Docker/webhook option (per README).
- **Triggers:** `on: pull_request:` quickstart with `fetch-depth: 0`
  checkout ([README](https://github.com/keithah/multi-provider-code-review)).
- **Providers:** `openrouter/<model>` incl. `openrouter/free` auto-route
  plus OAuth-CLI `claude/` `codex/` `gemini/` `opencode/` models, provider
  rotation/retries, `BUDGET_MAX_USD` guard, consensus inline comments
  (severity threshold + min-agreement), incremental review, SARIF+JSON —
  per [README](https://github.com/keithah/multi-provider-code-review)
  and [action.yml](https://raw.githubusercontent.com/keithah/multi-provider-code-review/main/action.yml).
- **Caveat:** 4★/1 fork at check — the least-mature pick; evaluate, don't
  adopt blindly.

### 6. `reviewdog/reviewdog` — comment plumbing, not a reviewer

"Provides a way to post review comments to code hosting services…
automatically by integrating with any linter tools… if findings are in
the diff" ([README](https://github.com/reviewdog/reviewdog)). MIT
([LICENSE](https://raw.githubusercontent.com/reviewdog/reviewdog/master/LICENSE)).
Accepts linter output from stdin (errorformat, RDFormat, checkstyle,
SARIF, diff) — an LLM step emits one of these and pipes it in; reporters
include `github-pr-review`, `github-pr-check`, `github-check`
([README](https://github.com/reviewdog/reviewdog)). Install via
[reviewdog/action-setup](https://github.com/reviewdog/action-setup);
canonical pattern `golint ./... | reviewdog
-f=golint -reporter=github-pr-review` from
[reviewdog.yml](https://raw.githubusercontent.com/reviewdog/reviewdog/master/.github/workflows/reviewdog.yml).
~9.6k stars; [v0.21.0 (2025-09-03)](https://raw.githubusercontent.com/reviewdog/reviewdog/master/CHANGELOG.md).
Use it when you build a custom LLM step and want diff-filtered inline
comments for free.

### 7. `BerriAI/litellm` — the multi-provider layer

"Open source AI Gateway… single, unified interface to call 100+ LLM
providers — OpenAI, Anthropic, Gemini, Bedrock, Azure, and more — using
the OpenAI format" ([README](https://github.com/BerriAI/litellm));
provider breadth in the [providers index](https://docs.litellm.ai/docs/providers).
MIT for content outside `enterprise/` (that directory has its own
license) —
[LICENSE](https://raw.githubusercontent.com/BerriAI/litellm/main/LICENSE).
Action pattern: run the proxy ([simple_proxy](https://docs.litellm.ai/docs/simple_proxy),
`config.yaml` per [configs](https://docs.litellm.ai/docs/proxy/configs))
and point the reviewer's `api_base` at it:

```python
from litellm import completion  # SDK form, from README
response = completion(model="anthropic/claude-sonnet-4-20250514",
                      messages=[{"role": "user", "content": "Hello!"}])
# via gateway: any OpenAI client at base_url="http://0.0.0.0:4000"
```

~58k stars; adopters include Stripe, Netflix ([README](https://github.com/BerriAI/litellm)).
pr-agent already embeds LiteLLM, so this matters most for custom builds
(crgpt's `OPENAI_API_ENDPOINT`, Tag1's `openai-compatible`, DIY Probot
apps).

### 8. `probot/probot` — DIY custom reviewer app

"A framework for building GitHub Apps in Node.js, written in
TypeScript" ([README](https://github.com/probot/probot)); "Apps… can be
installed directly on organizations and user accounts and granted access
to specific repositories… granular permissions and built-in webhooks"
([README](https://github.com/probot/probot),
[docs](https://probot.github.io/docs/)). ISC
([LICENSE](https://raw.githubusercontent.com/probot/probot/master/LICENSE)).
`app.on(event, cb)` handles webhooks (`issues.opened`-style; same
pattern for `pull_request` events) with `context.octokit` as the
authenticated client —
[webhooks](https://raw.githubusercontent.com/probot/probot/master/docs/webhooks.md),
[hello-world](https://raw.githubusercontent.com/probot/probot/master/docs/hello-world.md).
Deploy as Node (`APP_ID`, `WEBHOOK_SECRET`, `PRIVATE_KEY`), serverless,
or inside Actions via `@probot/adapter-github-actions` —
[deployment](https://raw.githubusercontent.com/probot/probot/master/docs/deployment.md).
~9.6k stars. Choose only if you need always-on app behavior none of the
above cover.

### 9. `All-Hands-AI/OpenHands` — agentic review, not a drop-in Action

MIT ([LICENSE](https://raw.githubusercontent.com/All-Hands-AI/OpenHands/main/LICENSE)).
PR review **is** a documented use case now, but as an Agent Canvas
automation: "GitHub PR Review Assistant — Automatically reviews pull
requests and posts feedback as a comment"
([pre-built automations](https://docs.openhands.dev/openhands/usage/agent-canvas/prebuilt-automations));
setup = GitHub token + GitHub MCP server, then start the workflow from
`Automate` (interviews you: which PRs, summary vs inline, CI, ignore
paths) —
[PR Review Assistant](https://docs.openhands.dev/openhands/usage/agent-canvas/prebuilt/github-pr-review).
No per-PR YAML exists. Heaviest option (hosting, secrets, MCP); consider
only if you already run OpenHands. The old
[`openhands-resolver`](https://github.com/All-Hands-AI/openhands-resolver)
is **not** a reviewer — it is label-triggered issue-fixing that
*responds to* review feedback
([resolver workflow](https://raw.githubusercontent.com/All-Hands-AI/openhands-resolver/main/examples/openhands-resolver.yml),
[pr-review-check.jinja](https://raw.githubusercontent.com/All-Hands-AI/openhands-resolver/main/openhands_resolver/prompts/guess_success/pr-review-check.jinja))
— and is archived/relocated.

## Excluded — not open source (considered, not researched deeply)

- **GitHub Copilot code review** — proprietary, part of the paid Copilot
  offering; fails the all-open-source constraint.
- **CodeRabbit** — closed-source commercial GitHub App
  ([coderabbit.ai](https://www.coderabbit.ai/)); its original OSS repo
  (`coderabbitai/ai-pr-reviewer`) returns HTTP 404 — only forks remain.
- **Gemini Code Assist / Google Cloud code review** — proprietary hosted
  product, not self-hostable OSS.
- **Sourcery, Greptile, Ellipsis and similar hosted reviewers** —
  proprietary SaaS; excluded on license grounds without deep research.
- **`sweepai/sweep`** — dropped on evidence, not just license suspicion:
  its [LICENSE](https://raw.githubusercontent.com/sweepai/sweep/main/LICENSE)
  is a proprietary Enterprise Edition license (production use needs a
  paid subscription; copying/merging/distributing forbidden), self-hosting
  needs a 14-day trial `LICENSE_KEY` per
  [deployment.mdx](https://raw.githubusercontent.com/sweepai/sweep/main/docs/pages/deployment.mdx),
  the product is issue→PR generation (not review; its
  [sweep.yml](https://raw.githubusercontent.com/sweepai/sweep/main/.github/workflows/sweep.yml)
  is entirely commented out), and the [README](https://raw.githubusercontent.com/sweepai/sweep/main/README.md)
  is now a pivot note to a JetBrains plugin (last push 2025-09-18).
- **`vercel-labs/openreview`** — calls itself "open-source, self-hosted"
  ([repo](https://github.com/vercel-labs/openreview)) but ships **no
  license file** (LICENSE 404s; API reports `license: null`), is
  Claude-only and comment-triggered (`@openreview`, no auto-review on
  open), and quiet since 2026-03-06 — excluded until licensed.
- **`sturdy-dev/codereview.gpt`** — MIT but a manual Chrome extension
  that "does not post comments on the PR page"
  ([README](https://github.com/sturdy-dev/codereview.gpt)); fails the
  automation criterion.

## Recommendation

**pr-agent (`The-PR-Agent/pr-agent`) as a GitHub Action, auto-review on
PR open, scaled via a central reusable workflow.**

- **Tool + deployment + trigger:** `the-pr-agent/pr-agent@main` in
  `.github/workflows/pr_agent.yml` with `on: pull_request: types:
  [opened, reopened, ready_for_review, synchronize]` (+ `issue_comment`
  for `/review` commands), `OPENAI_KEY`/`GITHUB_TOKEN` from secrets —
  snippet in §1, source
  [installation/github.md](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/docs/docs/installation/github.md).
  One repo holds the full workflow as `on: workflow_call:`; every other
  repo gets a thin caller —
  [reusable-workflows docs](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows).
- **Why it fits "multiple LLM providers, all open source, whole
  account":** MIT license; LiteLLM built in, so switching providers is a
  config change (OpenAI, Anthropic, Gemini, custom `api_base`,
  Ollama/VLLM — [model doc](https://raw.githubusercontent.com/qodo-ai/pr-agent/main/docs/docs/usage-guide/changing_a_model.md));
  auto-review on open plus comment commands; most active of the set
  ([v0.45.0, 2026-09-05](https://github.com/The-PR-Agent/pr-agent/releases/tag/v0.45.0)).
  Only the LLM API calls are paid/proprietary — everything else is OSS.
- **Runner-up:** cr-gpt in server mode — one Probot app installed on "All
  repositories" covers the whole account with zero per-repo files, at the
  cost of narrower providers and running a small server (use
  `OPENAI_API_ENDPOINT` to keep multi-provider via a LiteLLM proxy).
- **If you standardize on Claude:** `anthropics/claude-code-action` with
  the `types: [opened]` + `/review-pr` workflow — MIT and first-party,
  but Claude-only.
- **Watch, don't adopt:** Tag1 (rich multi-agent MIT action, no app
  mode) and keithah (consensus multi-provider, too young at 4★).
