if [ -f .env ]; then
  set -a
  . ./.env
  set +a
fi

: "${ANTHROPIC_API_KEY:?Set ANTHROPIC_API_KEY in your environment or .env file before running this script}"
export ANTHROPIC_MODEL="${ANTHROPIC_MODEL:-claude-3-5-haiku-20241022}"

curl -sS -D - \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  https://api.anthropic.com/v1/messages \
  -d '{
    "model": "'$ANTHROPIC_MODEL'",
    "max_tokens": 1,
    "messages": [
      { "role": "user", "content": "hi" }
    ]
  }'