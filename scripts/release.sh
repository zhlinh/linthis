#!/usr/bin/env bash
# Release script for linthis
# Usage: ./scripts/release.sh <version> [--push]
#        ./scripts/release.sh --patch|--minor|--major [--push]
#
# Examples:
#   ./scripts/release.sh 3.1.0
#   ./scripts/release.sh --patch        # 3.0.2 -> 3.0.3
#   ./scripts/release.sh --minor        # 3.0.2 -> 3.1.0
#   ./scripts/release.sh --major        # 3.0.2 -> 4.0.0
#   ./scripts/release.sh --patch --push # bump and push directly

set -eu

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

# Get current version from Cargo.toml
get_current_version() {
	grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'
}

# Bump version based on type
bump_version() {
	local current="$1"
	local bump_type="$2"

	IFS='.' read -r major minor patch <<<"$current"

	case "$bump_type" in
	--major)
		echo "$((major + 1)).0.0"
		;;
	--minor)
		echo "${major}.$((minor + 1)).0"
		;;
	--patch)
		echo "${major}.${minor}.$((patch + 1))"
		;;
	*)
		echo "$bump_type"
		;;
	esac
}

# Validate version format
validate_version() {
	local version="$1"
	if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
		echo -e "${RED}Error: Invalid version format '$version'. Expected: X.Y.Z${NC}"
		exit 1
	fi
}

# Update version in a file
update_version_in_file() {
	local file="$1"
	local old_version="$2"
	local new_version="$3"

	if [[ -f "$file" ]]; then
		# Use different sed syntax for macOS vs Linux
		if [[ "$OSTYPE" == "darwin"* ]]; then
			sed -i '' "s/^version = \"$old_version\"/version = \"$new_version\"/" "$file"
		else
			sed -i "s/^version = \"$old_version\"/version = \"$new_version\"/" "$file"
		fi
		echo -e "  ${GREEN}Updated${NC} $file"
	fi
}

main() {
	if [[ $# -eq 0 ]] || [[ "$1" == "-h" ]] || [[ "$1" == "--help" ]]; then
		echo "Usage: $0 <version>|--patch|--minor|--major [--push]"
		echo ""
		echo "Options:"
		echo "  <version>   Specific version (e.g., 3.1.0)"
		echo "  --patch     Bump patch version (3.0.2 -> 3.0.3)"
		echo "  --minor     Bump minor version (3.0.2 -> 3.1.0)"
		echo "  --major     Bump major version (3.0.2 -> 4.0.0)"
		echo "  --push      Auto commit, tag and push (skip interactive prompt)"
		echo ""
		echo "Current version: $(get_current_version)"
		exit 0
	fi

	# Parse --push flag
	local auto_push=false
	local version_arg="$1"
	if [[ $# -ge 2 ]] && [[ "$2" == "--push" ]]; then
		auto_push=true
	fi

	local current_version
	current_version=$(get_current_version)

	local new_version
	new_version=$(bump_version "$current_version" "$version_arg")

	validate_version "$new_version"

	if [[ "$current_version" == "$new_version" ]]; then
		echo -e "${YELLOW}Version is already $new_version${NC}"
		exit 0
	fi

	echo -e "${GREEN}Releasing linthis${NC}"
	echo -e "  Current version: ${YELLOW}$current_version${NC}"
	echo -e "  New version:     ${GREEN}$new_version${NC}"
	echo ""

	# Update versions in config files
	echo "Updating version in config files..."
	update_version_in_file "Cargo.toml" "$current_version" "$new_version"
	update_version_in_file "pyproject.toml" "$current_version" "$new_version"

	# Sync Cargo.lock
	echo ""
	echo "Syncing Cargo.lock..."
	cargo update -p linthis
	echo -e "  ${GREEN}Updated${NC} Cargo.lock"

	echo ""
	echo -e "${GREEN}Release preparation complete!${NC}"
	echo ""
	echo "Next steps:"
	echo "  1. Review changes: git diff"
	echo "  2. Commit: git add Cargo.toml pyproject.toml Cargo.lock && git commit -m 'chore: release v$new_version'"
	echo "  3. Tag: git tag v$new_version"
	echo "  4. Push: git push && git push origin v$new_version"
	echo ""
	echo "One-liner:"
	echo "  git add Cargo.toml pyproject.toml Cargo.lock && git commit -m 'chore: release v$new_version' && git tag v$new_version && git push && git push origin v$new_version"
	echo ""

	# Determine whether to push
	local do_push=false
	if [[ "$auto_push" == true ]]; then
		do_push=true
	else
		read -rp "Do you want to commit and push now? [y/N] " response
		case "$response" in
		[yY] | [yY][eE][sS])
			do_push=true
			;;
		esac
	fi

	if [[ "$do_push" == true ]]; then
		echo ""
		echo "Committing changes..."
		git add Cargo.toml pyproject.toml Cargo.lock
		git commit -m "chore: release v$new_version"

		echo "Creating tag v$new_version..."
		git tag "v$new_version"

		echo "Pushing to remote..."
		git push
		git push origin "v$new_version"

		echo ""
		echo -e "${GREEN}Release v$new_version pushed successfully!${NC}"
	else
		echo -e "${YELLOW}Skipped. Run the commands above manually when ready.${NC}"
	fi
}

main "$@"
