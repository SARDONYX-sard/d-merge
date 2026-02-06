import { createFileRoute } from '@tanstack/react-router';
import { Convert } from '@/components/templates/Convert';

export const Route = createFileRoute('/convert')({
  component: Convert,
});
