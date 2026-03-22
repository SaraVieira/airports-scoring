import { n as TSS_SERVER_FUNCTION, r as getServerFnById, t as createServerFn } from "../server.js";
import { createFileRoute, lazyRouteComponent } from "@tanstack/react-router";
//#region node_modules/.pnpm/@tanstack+start-server-core@1.167.1/node_modules/@tanstack/start-server-core/dist/esm/createSsrRpc.js
var createSsrRpc = (functionId, importer) => {
	const url = "/_serverFn/" + functionId;
	const serverFnMeta = { id: functionId };
	const fn = async (...args) => {
		return (importer ? await importer() : await getServerFnById(functionId))(...args);
	};
	return Object.assign(fn, {
		url,
		serverFnMeta,
		[TSS_SERVER_FUNCTION]: true
	});
};
//#endregion
//#region app/routes/airport.$iata.tsx
var $$splitComponentImporter = () => import("./airport._iata-DSFA3kE2.js");
var getAirport = createServerFn({ method: "GET" }).inputValidator((iata) => iata.toUpperCase()).handler(createSsrRpc("a7899a09f8826539179c9ce57da0b073e046f507d82c452334188e7f2194b86f"));
var Route = createFileRoute("/airport/$iata")({
	loader: ({ params }) => getAirport({ data: params.iata }),
	component: lazyRouteComponent($$splitComponentImporter, "component")
});
//#endregion
export { Route as t };
