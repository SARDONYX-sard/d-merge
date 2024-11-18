const urlPattern = /[a-zA-Z0-9/:.?&=_%-]/;
const urlPrefix = /^(https?:\/\/)[a-zA-Z0-9]/;

/**
 * Get url from one line.
 * @param line
 * @param cursorPosition - line number (starts at 1)
 * @returns
 */
export const extractUrlFromLine = (line: string, cursorPosition: number): string | undefined => {
  let start = cursorPosition - 1;
  let end = cursorPosition - 1;

  // Skip until url
  while (start > 0 && urlPattern.test(line[start - 1])) {
    start--;
  }
  while (end < line.length && urlPattern.test(line[end])) {
    end++;
  }

  const extracted = line.substring(start, end);

  // Simple url judgement
  if (urlPrefix.test(extracted)) {
    return extracted;
  }

  return undefined;
};
