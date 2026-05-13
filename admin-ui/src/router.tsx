import { createRootRoute, createRoute, createRouter, Outlet } from '@tanstack/react-router'
import Layout from './components/Layout'
import Login from './pages/Login'
import Callback from './pages/Callback'
import UserList from './pages/users/UserList'
import UserDetail from './pages/users/UserDetail'
import AppList from './pages/applications/AppList'
import AppForm from './pages/applications/AppForm'
import RoleList from './pages/roles/RoleList'
import RoleDetail from './pages/roles/RoleDetail'
import IdpList from './pages/idp/IdpList'
import IdpForm from './pages/idp/IdpForm'

const rootRoute = createRootRoute({ component: Outlet })

const loginRoute = createRoute({ getParentRoute: () => rootRoute, path: '/login', component: Login })
const callbackRoute = createRoute({ getParentRoute: () => rootRoute, path: '/callback', component: Callback })

const layoutRoute = createRoute({ getParentRoute: () => rootRoute, id: '_layout', component: Layout })

const indexRoute = createRoute({
  getParentRoute: () => layoutRoute,
  path: '/',
  component: UserList,
})

const usersRoute = createRoute({ getParentRoute: () => layoutRoute, path: '/users', component: UserList })
const userDetailRoute = createRoute({
  getParentRoute: () => layoutRoute,
  path: '/users/$userId',
  component: UserDetail,
})

const appsRoute = createRoute({ getParentRoute: () => layoutRoute, path: '/applications', component: AppList })
const appNewRoute = createRoute({ getParentRoute: () => layoutRoute, path: '/applications/new', component: AppForm })
const appEditRoute = createRoute({
  getParentRoute: () => layoutRoute,
  path: '/applications/$appId/edit',
  component: AppForm,
})

const rolesRoute = createRoute({ getParentRoute: () => layoutRoute, path: '/roles', component: RoleList })
const roleDetailRoute = createRoute({
  getParentRoute: () => layoutRoute,
  path: '/roles/$roleId',
  component: RoleDetail,
})

const idpRoute = createRoute({ getParentRoute: () => layoutRoute, path: '/idp', component: IdpList })
const idpNewRoute = createRoute({ getParentRoute: () => layoutRoute, path: '/idp/new', component: IdpForm })
const idpEditRoute = createRoute({
  getParentRoute: () => layoutRoute,
  path: '/idp/$idpId/edit',
  component: IdpForm,
})

const routeTree = rootRoute.addChildren([
  loginRoute,
  callbackRoute,
  layoutRoute.addChildren([
    indexRoute,
    usersRoute,
    userDetailRoute,
    appsRoute,
    appNewRoute,
    appEditRoute,
    rolesRoute,
    roleDetailRoute,
    idpRoute,
    idpNewRoute,
    idpEditRoute,
  ]),
])

export const router = createRouter({ routeTree, basepath: '/admin' })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
