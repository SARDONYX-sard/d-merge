// "oxc.configPath': "./vite.config.ts"
//
// is a workaround to resolve an issue where this setting does not work in VS Code because it is located within the `fmt` key
import * as config from './vite.config.ts';

export default config.default.lint;
