import { describe, expect, it } from 'vitest';
import { hkannoFromText, NULL_STR } from './hkanno';

describe('hkannoFromText', () => {
  it('parses multiple tracks correctly with trackName', () => {
    const text = `
# numOriginalFrames: 38
# duration: 1.5
# numAnnotationTracks: 2

trackName: Track1
# numAnnotations: 2
0.100000 MCO_DodgeOpen
0.400000 MCO_DodgeClose

trackName: Track2
# numAnnotations: 1
0.900000 MCO_Recovery
`;

    const tracks = hkannoFromText(text);
    expect(tracks.length).toBe(2);

    expect(tracks[0].track_name).toBe('Track1');
    expect(tracks[0].annotations).toEqual([
      { time: 0.1, text: 'MCO_DodgeOpen' },
      { time: 0.4, text: 'MCO_DodgeClose' },
    ]);

    expect(tracks[1].track_name).toBe('Track2');
    expect(tracks[1].annotations).toEqual([{ time: 0.9, text: 'MCO_Recovery' }]);
  });

  it('handles NULL_STR as null and trims text', () => {
    const text = `
trackName: Track1
# numAnnotations: 2
0.000000    ${NULL_STR}
0.500000   EventName
`;

    const tracks = hkannoFromText(text);
    expect(tracks.length).toBe(1);
    expect(tracks[0].annotations[0].text).toBeNull();
    expect(tracks[0].annotations[1].text).toBe('EventName');
  });

  it('handles tracks with flexible spacing in trackName', () => {
    const text = `
trackName                        :                Hi
# numAnnotations: 1
0.123456 SomeEvent
`;

    const tracks = hkannoFromText(text);
    expect(tracks.length).toBe(1);
    expect(tracks[0].track_name).toBe('Hi');
    expect(tracks[0].annotations[0].text).toBe('SomeEvent');
  });

  it('creates UNKNOWN track if annotations appear before trackName', () => {
    const text = `
0.111111 OrphanEvent
`;

    const tracks = hkannoFromText(text);
    expect(tracks.length).toBe(1);
    expect(tracks[0].track_name).toBe(NULL_STR);
    expect(tracks[0].annotations[0].text).toBe('OrphanEvent');
  });

  it('ignores extra comments and empty lines', () => {
    const text = `

# Some comment
trackName: TrackA

# numAnnotations: 2

0.100000 Event1
0.200000 Event2

# End of track
`;
    const tracks = hkannoFromText(text);
    expect(tracks.length).toBe(1);
    expect(tracks[0].track_name).toBe('TrackA');
    expect(tracks[0].annotations.length).toBe(2);
  });

  it('handles multiple spaces and tabs', () => {
    const text = `
trackName:\tSpacedTrack
# numAnnotations: 2
0.111111\t\tEvent  A
0.222222      EventB
`;
    const tracks = hkannoFromText(text);
    expect(tracks[0].track_name).toBe('SpacedTrack');
    expect(tracks[0].annotations[0].text).toBe('Event  A');
    expect(tracks[0].annotations[1].text).toBe('EventB');
  });

  it('handles zero-annotation tracks', () => {
    const text = `
trackName: EmptyTrack
# numAnnotations: 0
`;
    const tracks = hkannoFromText(text);
    expect(tracks.length).toBe(1);
    expect(tracks[0].track_name).toBe('EmptyTrack');
    expect(tracks[0].annotations.length).toBe(0);
  });

  it('handles NULL_STR with spaces around', () => {
    const text = `
trackName: NullTrack
# numAnnotations: 1
0.123456   ${NULL_STR}
`;
    const tracks = hkannoFromText(text);
    expect(tracks[0].annotations[0].text).toBeNull();
  });

  it('handles mixed-case trackName prefix', () => {
    const text = `
TRACKNAME: MixedCase
0.1 EventX
`;
    const tracks = hkannoFromText(text);
    expect(tracks[0].track_name).toBe('MixedCase');
    expect(tracks[0].annotations[0].text).toBe('EventX');
  });
});
