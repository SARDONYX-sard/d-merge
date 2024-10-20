import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';

import { Help } from './Help';

describe('Help Component', () => {
  const version = '1.0.0';

  beforeEach(() => {
    render(<Help href='https://github.com' version={version} />);
  });

  it('should render the version correctly', () => {
    expect(screen.getByText(`Version: ${version}`)).toBeInTheDocument();
  });

  it('should render the GitHub link correctly', () => {
    const linkElement = screen.getByRole('link', { name: 'GitHub' });

    expect(linkElement).toBeInTheDocument();
    expect(linkElement).toHaveAttribute('href', 'https://github.com');
    expect(linkElement).toHaveAttribute('target', '_blank');
    expect(linkElement).toHaveAttribute('rel', 'noopener');
  });

  it('should apply the correct styles to the Box component', () => {
    const boxElement = screen.getByText(`Version: ${version}`).parentElement;
    // console.log(prettyDOM(boxElement)); // Show HTML for debug

    expect(boxElement).toHaveStyle({
      display: 'flex',
      alignItems: 'center',
      flexDirection: 'column',
      height: '100%',
      justifyContent: 'space-evenly',
    });
  });
});
