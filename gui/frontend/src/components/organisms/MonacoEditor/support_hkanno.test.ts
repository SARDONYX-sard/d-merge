import { describe, expect, it } from 'vitest';
import { formatHkannoText, parseAnnotationLineSimple, parseHkannoText } from './support_hkanno';

describe('parseAnnotationLineSimple', () => {
  it('parses a valid annotation line', () => {
    const line = '  0.25 This is text';
    const parsed = parseAnnotationLineSimple(line);
    expect(parsed).not.toBeNull();
    expect(parsed).toBe(0.25);
  });

  it('returns null for comment line', () => {
    expect(parseAnnotationLineSimple('# comment')).toBeNull();
  });

  it('returns null for invalid time', () => {
    expect(parseAnnotationLineSimple('abc Some text')).toBeNull();
  });
});

describe('formatHkannoText', () => {
  it('removes leading spaces and preserves <time> <text>', () => {
    const text = '   0.5 Hello\n# numOriginalFrames: 10';
    const formatted = formatHkannoText(text);
    expect(formatted).toBe('0.5 Hello\n# numOriginalFrames: 10');
  });
});

describe('parseHkannoText', () => {
  it('parses meta lines, tracks, and annotations', () => {
    const text = `
# numOriginalFrames: 10
# duration: 1.5
# numAnnotationTracks: 2
0.0 first annotation
0.5 second annotation
# numAnnotations: 1
1.0 third annotation
`;

    const { meta, tracks, validations } = parseHkannoText(text);
    expect(meta.numOriginalFrames).toBe(10);
    expect(meta.duration).toBe(1.5);
    expect(meta.numAnnotationTracks).toBe(2);

    expect(tracks.length).toBe(2);
    expect(tracks[0].annotations.length).toBe(2);
    expect(tracks[0].annotations[0].time).toBe(0.0);
    expect(tracks[0].annotations[0].text).toBe('first annotation');

    // validations should be empty
    expect(validations.length).toBe(0);
  });

  it('records validation for invalid annotation', () => {
    const text = 'invalidTime some text';
    const { validations } = parseHkannoText(text);
    expect(validations.length).toBe(1);
    expect(validations[0].message).toContain('Time must be a valid number');
  });
});
