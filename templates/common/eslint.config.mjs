import js from "@eslint/js";
import tseslint from "@typescript-eslint/eslint-plugin";
import robloxTs from "eslint-plugin-roblox-ts";
import prettier from "eslint-plugin-prettier/recommended";

export default [
	{
		ignores: ["out/**"],
	},
	js.configs.recommended,
	...tseslint.configs["flat/recommended"],
	robloxTs.configs.recommended,
	prettier,
	{
		rules: {
			"prettier/prettier": "warn",
		},
	},
];
