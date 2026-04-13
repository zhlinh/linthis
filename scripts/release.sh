#!/usr/bin/env bash
# Release script for linthis
# Usage: ./scripts/release.sh <version> [--push]
#        ./scripts/release.sh --patch|--minor|--major [--push]
#        ./scripts/release.sh --changelog        # only regenerate CHANGELOG.md
#
# Examples:
#   ./scripts/release.sh 3.1.0
#   ./scripts/release.sh --patch        # 3.0.2 -> 3.0.3
#   ./scripts/release.sh --minor        # 3.0.2 -> 3.1.0
#   ./scripts/release.sh --major        # 3.0.2 -> 4.0.0
#   ./scripts/release.sh --patch --push # bump and push directly
#   ./scripts/release.sh --changelog    # regenerate CHANGELOG from git log only

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

# Generate a CHANGELOG section for the given version range.
# Args: $1 = version (e.g. "0.19.0"), $2 = date (YYYY-MM-DD),
#       $3 = git range (e.g. "v0.18.0..v0.19.0" or "v0.0.1..HEAD")
generate_changelog_section() {
	local version="$1"
	local date="$2"
	local range="$3"

	# Collect commits in conventional-commit categories
	# Skip release commits and merge commits
	local commits
	commits=$(git log --no-merges --pretty=format:"%s" "$range" 2>/dev/null |
		grep -v "^chore: release " || true)

	if [[ -z "$commits" ]]; then
		return 0
	fi

	echo "## [$version] - $date"
	echo ""

	# Categories: Added (feat), Fixed (fix), Performance (perf),
	# Changed (refactor), Documentation (docs), Other
	local feat_lines fix_lines perf_lines refactor_lines docs_lines other_lines
	feat_lines=$(echo "$commits" | grep -E "^feat(\(.*\))?:" || true)
	fix_lines=$(echo "$commits" | grep -E "^fix(\(.*\))?:" || true)
	perf_lines=$(echo "$commits" | grep -E "^perf(\(.*\))?:" || true)
	refactor_lines=$(echo "$commits" | grep -E "^refactor(\(.*\))?:" || true)
	docs_lines=$(echo "$commits" | grep -E "^docs(\(.*\))?:" || true)
	other_lines=$(echo "$commits" | grep -vE "^(feat|fix|perf|refactor|docs|chore|style|test|build|ci)(\(.*\))?:" || true)

	if [[ -n "$feat_lines" ]]; then
		echo "### Added"
		echo ""
		echo "$feat_lines" | sed -E 's/^feat(\([^)]+\))?: */- /'
		echo ""
	fi
	if [[ -n "$fix_lines" ]]; then
		echo "### Fixed"
		echo ""
		echo "$fix_lines" | sed -E 's/^fix(\([^)]+\))?: */- /'
		echo ""
	fi
	if [[ -n "$perf_lines" ]]; then
		echo "### Performance"
		echo ""
		echo "$perf_lines" | sed -E 's/^perf(\([^)]+\))?: */- /'
		echo ""
	fi
	if [[ -n "$refactor_lines" ]]; then
		echo "### Changed"
		echo ""
		echo "$refactor_lines" | sed -E 's/^refactor(\([^)]+\))?: */- /'
		echo ""
	fi
	if [[ -n "$docs_lines" ]]; then
		echo "### Documentation"
		echo ""
		echo "$docs_lines" | sed -E 's/^docs(\([^)]+\))?: */- /'
		echo ""
	fi
	if [[ -n "$other_lines" ]]; then
		echo "### Other"
		echo ""
		echo "$other_lines" | sed 's/^/- /'
		echo ""
	fi
}

# Update CHANGELOG.md by inserting a new version section after [Unreleased].
# Args: $1 = version, $2 = git range
update_changelog() {
	local version="$1"
	local range="$2"
	local date
	date=$(date +%Y-%m-%d)

	local section
	section=$(generate_changelog_section "$version" "$date" "$range")

	if [[ -z "$section" ]]; then
		echo -e "  ${YELLOW}No changelog entries for $version (no relevant commits)${NC}"
		return 0
	fi

	local changelog="CHANGELOG.md"
	if [[ ! -f "$changelog" ]]; then
		cat >"$changelog" <<EOF
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

EOF
	fi

	# Insert section after the "## [Unreleased]" line via temp file
	local sect_file tmp
	sect_file=$(mktemp)
	tmp=$(mktemp)
	echo "$section" >"$sect_file"
	awk -v sect_file="$sect_file" '
		/^## \[Unreleased\]/ {
			print
			print ""
			while ((getline line < sect_file) > 0) print line
			close(sect_file)
			next
		}
		{ print }
	' "$changelog" >"$tmp"
	mv "$tmp" "$changelog"
	rm -f "$sect_file"

	echo -e "  ${GREEN}Updated${NC} $changelog"
}

# Regenerate full CHANGELOG.md from git history (release commits as boundaries).
# Walks through "chore: release vX.Y.Z" commits in chronological order.
regenerate_changelog() {
	echo "Regenerating CHANGELOG.md from git history..."

	# Find all release commits in chronological order (oldest first)
	# Use TAB separator to handle commit subjects with spaces
	local releases
	releases=$(git log --reverse \
		--pretty=format:"%H%x09%ad%x09%s" \
		--date=short \
		--grep="^chore: release v[0-9]" 2>/dev/null)

	if [[ -z "$releases" ]]; then
		echo -e "  ${YELLOW}No release commits found.${NC}"
		return 0
	fi

	# Write header
	cat >CHANGELOG.md <<'EOF'
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

EOF

	# Collect all sections newest-first
	local all_sections_file
	all_sections_file=$(mktemp)

	# Read all releases into arrays first
	local -a shas dates versions
	while IFS=$'\t' read -r sha date_part subject; do
		[[ -z "$sha" ]] && continue
		local version
		version=$(echo "$subject" | sed -E 's/^chore: release v//')
		shas+=("$sha")
		dates+=("$date_part")
		versions+=("$version")
	done <<<"$releases"

	local first_sha
	first_sha=$(git rev-list --max-parents=0 HEAD | head -1)

	# Generate sections from newest to oldest, prepending each to the file
	local i count=${#shas[@]}
	for ((i = count - 1; i >= 0; i--)); do
		local sha="${shas[$i]}"
		local date_part="${dates[$i]}"
		local version="${versions[$i]}"
		local range
		if [[ $i -eq 0 ]]; then
			range="${first_sha}..${sha}"
		else
			range="${shas[$((i - 1))]}..${sha}"
		fi
		generate_changelog_section "$version" "$date_part" "$range" >>"$all_sections_file"
	done

	# Append sections to CHANGELOG.md after [Unreleased]
	local tmp
	tmp=$(mktemp)
	awk -v sect_file="$all_sections_file" '
		/^## \[Unreleased\]/ {
			print
			print ""
			while ((getline line < sect_file) > 0) print line
			close(sect_file)
			next
		}
		{ print }
	' CHANGELOG.md >"$tmp"
	mv "$tmp" CHANGELOG.md
	rm -f "$all_sections_file"

	echo -e "  ${GREEN}Regenerated${NC} CHANGELOG.md"
}

main() {
	if [[ $# -eq 0 ]] || [[ "$1" == "-h" ]] || [[ "$1" == "--help" ]]; then
		echo "Usage: $0 <version>|--patch|--minor|--major [--push]"
		echo "       $0 --changelog"
		echo ""
		echo "Options:"
		echo "  <version>     Specific version (e.g., 3.1.0)"
		echo "  --patch       Bump patch version (3.0.2 -> 3.0.3)"
		echo "  --minor       Bump minor version (3.0.2 -> 3.1.0)"
		echo "  --major       Bump major version (3.0.2 -> 4.0.0)"
		echo "  --push        Auto commit, tag and push (skip interactive prompt)"
		echo "  --changelog   Regenerate full CHANGELOG.md from git history"
		echo ""
		echo "Current version: $(get_current_version)"
		exit 0
	fi

	# --changelog: only regenerate CHANGELOG.md, no version bump
	if [[ "$1" == "--changelog" ]]; then
		regenerate_changelog
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

	# Update CHANGELOG.md with commits since last release
	echo ""
	echo "Updating CHANGELOG.md..."
	local last_release_sha
	last_release_sha=$(git log --pretty=format:"%H" --grep="^chore: release v[0-9]" -n 1 2>/dev/null || true)
	local range
	if [[ -n "$last_release_sha" ]]; then
		range="${last_release_sha}..HEAD"
	else
		# No previous release commit — use first commit
		local first_sha
		first_sha=$(git rev-list --max-parents=0 HEAD | head -1)
		range="${first_sha}..HEAD"
	fi
	update_changelog "$new_version" "$range"

	echo ""
	echo -e "${GREEN}Release preparation complete!${NC}"
	echo ""
	echo "Next steps:"
	echo "  1. Review changes: git diff"
	echo "  2. Commit: git add Cargo.toml pyproject.toml Cargo.lock CHANGELOG.md && git commit -m 'chore: release v$new_version'"
	echo "  3. Tag: git tag v$new_version"
	echo "  4. Push: git push origin master && git push origin v$new_version"
	echo ""
	echo "One-liner:"
	echo "  git add Cargo.toml pyproject.toml Cargo.lock CHANGELOG.md && git commit -m 'chore: release v$new_version' && git tag v$new_version && git push origin master && git push origin v$new_version"
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
		git add Cargo.toml pyproject.toml Cargo.lock CHANGELOG.md
		git commit -m "chore: release v$new_version"

		echo "Creating tag v$new_version..."
		git tag "v$new_version"

		echo "Pushing to remote..."
		git push origin master
		git push origin "v$new_version"

		echo ""
		echo -e "${GREEN}Release v$new_version pushed successfully!${NC}"
	else
		echo -e "${YELLOW}Skipped. Run the commands above manually when ready.${NC}"
	fi
}

main "$@"
