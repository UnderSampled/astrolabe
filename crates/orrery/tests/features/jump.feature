@jump @functional
Feature: Jumping in the manor
  Background:
    Given Hype is ready to move in a clear part of the manor

  Scenario: Jumping from a standstill and landing
    Given Hype has backed into a clear grounded jump start
    When the player presses and releases jump while standing
    Then Hype rises above the floor during the jump
    And Hype returns to the starting floor and settles
