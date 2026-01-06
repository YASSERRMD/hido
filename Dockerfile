# Build Stage
FROM rust:1.75-slim-bookworm as builder

# Install Python and build tools
RUN apt-get update && apt-get install -y python3 python3-pip python3-venv git && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create virtual environment for maturin
RUN python3 -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

# Install maturin
RUN pip install maturin

# Copy sources
COPY . .

# Build wheels
# We build for release to get optimized bindings
RUN maturin build --release --out dist

# Runtime Stage
FROM python:3.9-slim-bookworm

WORKDIR /app

# Copy artifacts from builder
COPY --from=builder /app/dist /app/dist
COPY --from=builder /app/python /app/python

# Install dependencies and the built wheel
# Note: urllib3<2 is required for some mac/ssl setups but fine on linux usually, 
# keeping it consistent with local dev or relaxing if needed. 
# We install the wheel found in /app/dist
RUN pip install --no-cache-dir dist/*.whl && \
    pip install --no-cache-dir gradio cohere python-dotenv "urllib3<2"

# Expose Gradio port
EXPOSE 7860

# Environment variables
ENV GRADIO_SERVER_NAME="0.0.0.0"

# Run the application
CMD ["python", "python/app.py"]
