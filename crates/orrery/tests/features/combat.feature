@combat @functional
Feature: Combat in the manor
  Background:
    Given Hype is ready to move in a clear part of the manor

  Scenario: Selecting the sword and attacking once
    Given Hype is grounded away from the manor save point
    When the player selects the sword and presses attack once
    Then Hype equips the sword and performs an attack action
    And Hype returns to armed idle without moving away

  Scenario: Launching one crossbow arrow initially
    Given Hype has three test arrows in the supplied manor state
    When the player selects the crossbow and presses attack once
    Then the compiled scripts spawn a model 733 arrow
    And the arrow moves between two initial observations
