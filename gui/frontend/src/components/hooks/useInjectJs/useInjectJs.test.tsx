import { render } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';

import { JsProvider } from '@/components/providers/JsProvider';
import { STORAGE } from '@/lib/storage';

import { useInjectJs } from '../useInjectJs';

const TestComponent = () => {
  useInjectJs();
  return <div>Test Component</div>;
};

const enableExecJs = () => localStorage.setItem('run-script', 'true');

// Test suite
describe('useInjectScript', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('should inject the script when `run-script` is true', () => {
    const jsCode = 'console.log("Test script loaded")';
    STORAGE.set('custom-js', jsCode);
    enableExecJs();

    const { unmount } = render(
      <JsProvider>
        <TestComponent />
      </JsProvider>,
    );

    expect(document.body.querySelector('script')).toBeInTheDocument();
    expect(document.body.querySelector('script')?.innerHTML).toBe(jsCode);

    unmount();

    // Cleanup check
    expect(document.body.querySelector('script')).not.toBeInTheDocument();
  });

  it('should not inject the script if `run-script` is not true', () => {
    const { unmount } = render(
      <JsProvider>
        <TestComponent />
      </JsProvider>,
    );

    expect(document.body.querySelector('script')).not.toBeInTheDocument();

    unmount();
  });

  it('should not inject the script again if already run', () => {
    enableExecJs();

    const { unmount } = render(
      <JsProvider>
        <TestComponent />
      </JsProvider>,
    );

    unmount();

    render(
      <JsProvider>
        <TestComponent />
      </JsProvider>,
    );

    expect(document.body.querySelector('script')).toBeInTheDocument();
    expect(document.body.querySelectorAll('script').length).toBe(1);
  });
});
