# =============================================================================
# Frontend Dockerfile - TanStack Start + Nitro (Node.js preset)
# Multi-stage build for production optimization
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Builder - Install dependencies and build
# -----------------------------------------------------------------------------
FROM node:22-alpine AS builder

WORKDIR /app

# Copy package files first for better layer caching
COPY package.json pnpm-lock.yaml* package-lock.json* yarn.lock* ./

# Install pnpm and dependencies
RUN corepack enable pnpm && \
    pnpm install --frozen-lockfile

# Copy source files
COPY . .

# Build the application
# TanStack Start with Nitro outputs to .output directory
RUN pnpm run build

# -----------------------------------------------------------------------------
# Stage 2: Runner - Minimal production image
# -----------------------------------------------------------------------------
FROM node:22-alpine AS runner

WORKDIR /app

# Create non-root user for security
RUN addgroup --system --gid 1001 nodejs && \
    adduser --system --uid 1001 tanstack

# Copy built output from Nitro
# Nitro generates a self-contained server in .output
COPY --from=builder --chown=tanstack:nodejs /app/.output ./.output

# Switch to non-root user
USER tanstack

# Expose port
EXPOSE 3000

# Environment variables
ENV NODE_ENV=production
ENV HOST=0.0.0.0
ENV PORT=3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3000/ || exit 1

# Start the Nitro server
# Nitro outputs the server entry point at .output/server/index.mjs
CMD ["node", ".output/server/index.mjs"]
