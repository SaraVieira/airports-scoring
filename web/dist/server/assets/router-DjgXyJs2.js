import { t as Route$5 } from "./airport._iata-V9-nTsQx.js";
import { HeadContent, Outlet, Scripts, createFileRoute, createRootRoute, createRouter, lazyRouteComponent } from "@tanstack/react-router";
import { jsx, jsxs } from "react/jsx-runtime";
//#region app/routes/__root.tsx
var Route$4 = createRootRoute({
	head: () => ({ meta: [
		{ charSet: "utf-8" },
		{
			name: "viewport",
			content: "width=device-width, initial-scale=1"
		},
		{ title: "Airport Intelligence" }
	] }),
	component: RootComponent
});
function RootComponent() {
	return /* @__PURE__ */ jsxs("html", {
		lang: "en",
		children: [/* @__PURE__ */ jsx("head", { children: /* @__PURE__ */ jsx(HeadContent, {}) }), /* @__PURE__ */ jsxs("body", { children: [/* @__PURE__ */ jsx(Outlet, {}), /* @__PURE__ */ jsx(Scripts, {})] })]
	});
}
//#endregion
//#region app/routes/search.tsx
var $$splitComponentImporter$3 = () => import("./search-CMvdcmJW.js");
var Route$3 = createFileRoute("/search")({ component: lazyRouteComponent($$splitComponentImporter$3, "component") });
//#endregion
//#region app/routes/operators.tsx
var $$splitComponentImporter$2 = () => import("./operators-BJ1_MY96.js");
var Route$2 = createFileRoute("/operators")({ component: lazyRouteComponent($$splitComponentImporter$2, "component") });
//#endregion
//#region app/routes/globe.tsx
var $$splitComponentImporter$1 = () => import("./globe-CNKPicUn.js");
var Route$1 = createFileRoute("/globe")({ component: lazyRouteComponent($$splitComponentImporter$1, "component") });
//#endregion
//#region app/routes/index.tsx
var $$splitComponentImporter = () => import("./routes-BYeCbVQy.js");
var Route = createFileRoute("/")({ component: lazyRouteComponent($$splitComponentImporter, "component") });
//#endregion
//#region app/routeTree.gen.ts
var SearchRoute = Route$3.update({
	id: "/search",
	path: "/search",
	getParentRoute: () => Route$4
});
var OperatorsRoute = Route$2.update({
	id: "/operators",
	path: "/operators",
	getParentRoute: () => Route$4
});
var GlobeRoute = Route$1.update({
	id: "/globe",
	path: "/globe",
	getParentRoute: () => Route$4
});
var rootRouteChildren = {
	IndexRoute: Route.update({
		id: "/",
		path: "/",
		getParentRoute: () => Route$4
	}),
	GlobeRoute,
	OperatorsRoute,
	SearchRoute,
	AirportIataRoute: Route$5.update({
		id: "/airport/$iata",
		path: "/airport/$iata",
		getParentRoute: () => Route$4
	})
};
var routeTree = Route$4._addFileChildren(rootRouteChildren)._addFileTypes();
//#endregion
//#region app/router.tsx
function getRouter() {
	return createRouter({
		routeTree,
		scrollRestoration: true
	});
}
//#endregion
export { getRouter };
