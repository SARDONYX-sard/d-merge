/** @type {import('next').NextConfig} */
const nextConfig = {
  basePath: '/frontend',
  distDir: './out',
  output: 'export',
  trailingSlash: true,
  reactStrictMode: true,
  experimental: {
    reactCompiler: true,
  },
  images: {
    unoptimized: true,
  },
  webpack: {
    __dirname: false,
  },
};

export default nextConfig;
