# =============================================================================
# Writing Style Service Dockerfile
# Using Debian slim for better native module compatibility
# =============================================================================

FROM node:20-slim

# Install dumb-init for proper signal handling
RUN apt-get update && apt-get install -y --no-install-recommends \
    dumb-init \
    wget \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Enable pnpm
RUN corepack enable pnpm

# Copy package files and .npmrc for pnpm config
COPY package.json pnpm-lock.yaml* .npmrc* ./

# Install dependencies - .npmrc allows better-sqlite3 to run build scripts
RUN pnpm install --frozen-lockfile

# Copy source code and build
COPY . .
RUN pnpm build

# Create non-root user for security
RUN groupadd --system --gid 1001 nodejs && \
    useradd --system --uid 1001 --gid nodejs appuser && \
    mkdir -p /app/data && chown -R appuser:nodejs /app/data

# Switch to non-root user
USER appuser

# Environment variables
ENV NODE_ENV=production
ENV PORT=3004
ENV HOST=0.0.0.0

# Expose port
EXPOSE 3004

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3004/health || exit 1

# Use dumb-init as entrypoint
ENTRYPOINT ["dumb-init", "--"]

CMD ["node", "dist/index.js"]
