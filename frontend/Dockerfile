# Use Node.js for development
FROM node:26

WORKDIR /app

# Copy package.json and lock file first to optimize caching
COPY package.json package-lock.json ./

# Install dependencies from the lockfile for reproducible image builds
RUN npm ci

# Copy the rest of the project files
COPY . .

RUN npm run build

# Install static server
RUN npm install -g serve

# Expose the Vite development server port (default is 5173)
EXPOSE 5173

# Start the frontend app
CMD ["serve", "-s", "dist", "-l", "5173"]
