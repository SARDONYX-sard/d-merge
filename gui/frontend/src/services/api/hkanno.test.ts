import { describe, expect, it } from 'vitest';
import { hkannoFromText, NULL_STR } from './hkanno';

describe('hkannoFromText', () => {
  it('parses multiple tracks correctly', () => {
    const text = `# numOriginalFrames: 38
# duration: 1.5
# numAnnotationTracks: 2
# numAnnotations: 2
0.100000 MCO_DodgeOpen
0.400000 MCO_DodgeClose
# numAnnotations: 1
0.900000 MCO_Recovery
`;

    const tracks = hkannoFromText(text);
    expect(tracks.length).toBe(2);

    expect(tracks[0].annotations).toEqual([
      { time: 0.1, text: 'MCO_DodgeOpen' },
      { time: 0.4, text: 'MCO_DodgeClose' },
    ]);
    expect(tracks[1].annotations).toEqual([{ time: 0.9, text: 'MCO_Recovery' }]);
  });

  it('handles NULL_STR as null', () => {
    const text = `# numOriginalFrames: 5
# duration: 0.5
# numAnnotationTracks: 1
# numAnnotations: 2
0.000000 ${NULL_STR}
0.500000 EventName
`;

    const tracks = hkannoFromText(text);
    expect(tracks[0].annotations[0].text).toBeNull();
    expect(tracks[0].annotations[1].text).toBe('EventName');
  });

  it('skips empty tracks', () => {
    const text = `# numOriginalFrames: 5
# duration: 0.5
# numAnnotationTracks: 2
# numAnnotations: 0
# numAnnotations: 1
0.200000 EventX
`;

    const tracks = hkannoFromText(text);
    expect(tracks.length).toBe(2);
    expect(tracks[1].annotations[0].text).toBe('EventX');
  });
});
