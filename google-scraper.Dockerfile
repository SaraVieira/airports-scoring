# Dockerfile for google-reviews-scraper-pro
# Selenium-based scraper with FastAPI REST API
FROM python:3.13-slim-bookworm

# Install Chrome dependencies for seleniumbase
RUN apt-get update && apt-get install -y --no-install-recommends \
    wget gnupg2 unzip \
    fonts-liberation libasound2 libatk-bridge2.0-0 libatk1.0-0 \
    libcups2 libdbus-1-3 libdrm2 libgbm1 libgtk-3-0 libnspr4 \
    libnss3 libx11-xcb1 libxcomposite1 libxdamage1 libxrandr2 \
    xdg-utils libxss1 libappindicator3-1 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Install Python dependencies
COPY google-reviews-scraper-pro/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Install Chrome via seleniumbase
RUN seleniumbase install chromedriver && seleniumbase install chrome

# Copy scraper code
COPY google-reviews-scraper-pro/ .

# Create config.yaml with our defaults
RUN echo "use_mongodb: false" > config.yaml

EXPOSE 8000

CMD ["uvicorn", "api_server:app", "--host", "0.0.0.0", "--port", "8000"]
