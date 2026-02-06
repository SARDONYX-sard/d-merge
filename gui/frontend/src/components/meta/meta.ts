import packageJson from '../../../../../package.json';

export const metadata = {
  title: packageJson.name,
  description: packageJson.description,
};

export const HELP_INFO = {
  homepage: packageJson.homepage,
  version: packageJson.version,
};
