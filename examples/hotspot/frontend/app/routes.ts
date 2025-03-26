import { type RouteConfig, index, route } from '@react-router/dev/routes';

export default [
  route('/guest/s/default', 'pages/portal.tsx'),
  route('success', 'pages/success.tsx'),
] satisfies RouteConfig;