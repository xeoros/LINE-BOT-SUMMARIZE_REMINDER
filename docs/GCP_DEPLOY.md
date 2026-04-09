# GCP Deploy

This repository can be built by Cloud Build using either `cloudbuild.yaml` or the root `Dockerfile`.

## 1. Create an Artifact Registry repository

```bash
gcloud artifacts repositories create line-bot \
  --repository-format=docker \
  --location=us-central1
```

## 2. Build the container with Cloud Build

```bash
gcloud builds submit \
  --config cloudbuild.yaml \
  --substitutions=_IMAGE_URI=us-central1-docker.pkg.dev/PROJECT_ID/line-bot/line-bot-summarize:latest
```

You can also let Cloud Build discover the `Dockerfile` automatically:

```bash
gcloud builds submit --tag us-central1-docker.pkg.dev/PROJECT_ID/line-bot/line-bot-summarize:latest
```

## 3. Deploy to Cloud Run

```bash
gcloud run deploy line-bot-summarize \
  --image us-central1-docker.pkg.dev/PROJECT_ID/line-bot/line-bot-summarize:latest \
  --region us-central1 \
  --platform managed \
  --allow-unauthenticated \
  --port 8080
```

## 4. Configure runtime environment variables

Set the required variables during deploy or afterward:

```bash
gcloud run services update line-bot-summarize \
  --region us-central1 \
  --set-env-vars PORT=8080,LOG_DIR=/tmp/logs,SCHEDULES_CONFIG_PATH=config/schedules.toml,DATABASE_URL=postgresql://USER:PASSWORD@HOST:5432/DB,AI_PROVIDER=claude,ENABLE_LINE=true,ENABLE_SLACK=false,ENABLE_TEAMS=false \
  --set-secrets LINE_CHANNEL_ACCESS_TOKEN=LINE_CHANNEL_ACCESS_TOKEN:latest,LINE_CHANNEL_SECRET=LINE_CHANNEL_SECRET:latest,CLAUDE_API_KEY=CLAUDE_API_KEY:latest
```

Adjust the secrets and flags to match the platforms you actually enable. If `ENABLE_SLACK=true` or `ENABLE_TEAMS=true`, their corresponding credentials are required at startup.

## Notes

- Cloud Run injects the `PORT` environment variable automatically; the app already listens on `0.0.0.0:$PORT`.
- `LOG_DIR` is set to `/tmp/logs` because Cloud Run does not provide persistent local storage.
- The app runs SQL migrations on startup, so the configured `DATABASE_URL` must be reachable from Cloud Run.
