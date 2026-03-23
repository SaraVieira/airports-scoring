# Dockerfile for google-reviews-scraper-pro
# Selenium-based scraper with FastAPI REST API
FROM python:3.13-slim-bookworm

# Install Chrome from Google's repo
RUN apt-get update && apt-get install -y --no-install-recommends \
    wget gnupg2 unzip \
    fonts-liberation libasound2 libatk-bridge2.0-0 libatk1.0-0 \
    libcups2 libdbus-1-3 libdrm2 libgbm1 libgtk-3-0 libnspr4 \
    libnss3 libx11-xcb1 libxcomposite1 libxdamage1 libxrandr2 \
    xdg-utils libxss1 libappindicator3-1 \
    && wget -q -O - https://dl.google.com/linux/linux_signing_key.pub | gpg --dearmor -o /usr/share/keyrings/google-chrome.gpg \
    && echo "deb [arch=amd64 signed-by=/usr/share/keyrings/google-chrome.gpg] http://dl.google.com/linux/chrome/deb/ stable main" > /etc/apt/sources.list.d/google-chrome.list \
    && apt-get update && apt-get install -y --no-install-recommends google-chrome-stable \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Install Python dependencies
COPY google-reviews-scraper-pro/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Install matching chromedriver via seleniumbase
RUN seleniumbase install chromedriver

# Copy scraper code
COPY google-reviews-scraper-pro/ .

# Create config.yaml with our defaults
RUN echo "use_mongodb: false" > config.yaml

EXPOSE 8000

CMD ["uvicorn", "api_server:app", "--host", "0.0.0.0", "--port", "8000"]
