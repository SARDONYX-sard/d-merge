import { describe, expect, it } from 'vitest';
import { parseHkannoLine } from '.';

describe('parseHkannoLine', () => {
  it('returns type=none for empty lines', () => {
    expect(parseHkannoLine('')).toEqual({ type: 'none', errors: [] });
    expect(parseHkannoLine('   ')).toEqual({ type: 'none', errors: [] });
  });

  it('parses meta lines (# key: value)', () => {
    const res = parseHkannoLine('# numOriginalFrames: 38');
    expect(res.type).toBe('meta');
    expect(res.eventName).toBe('numOriginalFrames');
    expect(res.rawText).toBe('38');
    expect(res.tokenPositions?.verb).toMatchObject({ startColumn: 3 });
  });

  it('parses plain text annotation lines', () => {
    const res = parseHkannoLine('0.100000 MCO_DodgeOpen');
    expect(res.type).toBe('text');
    expect(res.time).toBeCloseTo(0.1);
    expect(res.eventName).toBe('MCO_DodgeOpen');
    expect(res.rawText).toBe('');
  });

  it('parses animmotion with 3 numeric args', () => {
    const res = parseHkannoLine('0.200000 AnimMotion 1.0 2.0 3.0');
    expect(res.type).toBe('motion');
    expect(res.eventName).toBe('AnimMotion');
    expect(res.args).toEqual([1, 2, 3]);
    expect(res.tokenPositions?.argPositions?.length).toBe(3);
  });

  it('parses animrotation with 1 numeric arg', () => {
    const res = parseHkannoLine('0.300000 AnimRotation 45.0');
    expect(res.type).toBe('rotation');
    expect(res.args).toEqual([45]);
  });

  it('reports error for animrotation without numeric value', () => {
    const res = parseHkannoLine('0.300000 AnimRotation');
    expect(res.type).toBe('invalid');
    expect(res.errors).toContain('animrotation missing numeric angle');
  });

  it('reports invalid time tokens', () => {
    const res = parseHkannoLine('abc MCO_Test');
    expect(res.type).toBe('invalid');
    expect(res.errors?.[0]).toMatch(/Invalid time token/);
  });

  it('preserves token positions', () => {
    const res = parseHkannoLine('0.123 AnimMotion 4.56 7.89 1.23');
    const pos = res.tokenPositions!;
    expect(pos.time).toMatchObject({ line: 1, startColumn: 1 });
    expect(pos.verb?.startColumn).toBeGreaterThan(pos.time.startColumn);
  });

  it('handles trailing spaces gracefully', () => {
    const res = parseHkannoLine('0.5 MCO_End   ');
    expect(res.type).toBe('text');
    expect(res.eventName).toBe('MCO_End');
  });
});
