import { createRootRoute, createRoute, createRouter, Outlet } from '@tanstack/react-router'
import Layout from './components/Layout'
import Login from './pages/Login'
import Callback from './pages/Callback'
import UserList from './pages/users/UserList'
import UserDetail from './pages/users/UserDetail'
import AppList from './pages/applications/AppList'
import AppForm from './pages/applications/AppForm'
import GroupList from './pages/groups/GroupList'
import GroupDetail from './pages/groups/GroupDetail'
import OperatorList from './pages/operators/OperatorList'
import OperatorDetail from './pages/operators/OperatorDetail'
import OperatorRoleList from './pages/operator-roles/OperatorRoleList'
import OperatorRoleForm from './pages/operator-roles/OperatorRoleForm'
import OperatorRoleDetail from './pages/operator-roles/OperatorRoleDetail'
import OperatorPermissionList from './pages/operator-permissions/OperatorPermissionList'
import TenantList from './pages/tenants/TenantList'
import TenantDetail from './pages/tenants/TenantDetail'
import ClaimDefList from './pages/claims/ClaimDefList'
import ClaimDefForm from './pages/claims/ClaimDefForm'
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

const groupsRoute = createRoute({ getParentRoute: () => layoutRoute, path: '/groups', component: GroupList })
const groupDetailRoute = createRoute({
  getParentRoute: () => layoutRoute,
  path: '/groups/$groupId',
  component: GroupDetail,
})

const claimsRoute = createRoute({ getParentRoute: () => layoutRoute, path: '/claims', component: ClaimDefList })
const claimNewRoute = createRoute({
  getParentRoute: () => layoutRoute,
  path: '/claims/new',
  component: ClaimDefForm,
})
const claimEditRoute = createRoute({
  getParentRoute: () => layoutRoute,
  path: '/claims/$claimId/edit',
  component: ClaimDefForm,
})
const tenantsRoute = createRoute({ getParentRoute: () => layoutRoute, path: '/tenants', component: TenantList })
const tenantDetailRoute = createRoute({
  getParentRoute: () => layoutRoute,
  path: '/tenants/$tenantId',
  component: TenantDetail,
})
const operatorsRoute = createRoute({ getParentRoute: () => layoutRoute, path: '/operators', component: OperatorList })
const operatorDetailRoute = createRoute({
  getParentRoute: () => layoutRoute,
  path: '/operators/$operatorId',
  component: OperatorDetail,
})
const operatorRolesRoute = createRoute({ getParentRoute: () => layoutRoute, path: '/operator-roles', component: OperatorRoleList })
const operatorRoleNewRoute = createRoute({
  getParentRoute: () => layoutRoute,
  path: '/operator-roles/new',
  component: OperatorRoleForm,
})
const operatorRoleDetailRoute = createRoute({
  getParentRoute: () => layoutRoute,
  path: '/operator-roles/$roleId',
  component: OperatorRoleDetail,
})
const operatorPermissionsRoute = createRoute({ getParentRoute: () => layoutRoute, path: '/operator-permissions', component: OperatorPermissionList })

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
    groupsRoute,
    groupDetailRoute,
    claimsRoute,
    claimNewRoute,
    claimEditRoute,
    tenantsRoute,
    tenantDetailRoute,
    operatorsRoute,
    operatorDetailRoute,
    operatorRolesRoute,
    operatorRoleNewRoute,
    operatorRoleDetailRoute,
    operatorPermissionsRoute,
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
