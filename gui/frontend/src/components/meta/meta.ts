import type { Metadata } from 'next';
import packageJson from '../../../../../package.json';

export const metadata: Metadata = {
  title: packageJson.name,
  description: packageJson.description,
};

export const HELP_INFO = {
  homepage: packageJson.homepage,
  version: packageJson.version,
};
